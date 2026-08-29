//! Optimizacion de juegos de PlayStation 3 sin cambiar su contenido.
//!
//! El perfil universal extrae y reconstruye un ISO estandar para retirar el
//! relleno del disco. El perfil RPCS3 conserva el archivo o carpeta tal cual y
//! pide al sistema de archivos que lo comprima de forma transparente.
//!
//! Aqui esta la parte delicada del programa. Hay juegos que llevan un indice de
//! sus propios archivos y se cuelgan si falta uno, asi que:
//!   * nunca se propone borrar nada del nucleo del juego,
//!   * hay una lista de archivos que se niega a borrar aunque se lo pidan,
//!   * y todo pasa por una vista previa antes de tocar el disco.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// (modo, herramienta)
pub const MODES: &[(&str, &str)] = &[
    ("ps3extract", "ps3iso"),
    ("ps3build", "ps3iso"),
    ("ps3split", "ps3iso"),
    ("ps3compact", "ps3iso"),
    ("ps3rpcs3", "rpcs3fs"),
];

pub fn is_mode(mode: &str) -> bool {
    MODES.iter().any(|m| m.0 == mode)
}

pub fn tool_for(mode: &str) -> Option<&'static str> {
    MODES.iter().find(|m| m.0 == mode).map(|m| m.1)
}

/// Ejecutable concreto que toca en cada modo.
pub fn exe_for(mode: &str) -> &'static str {
    match mode {
        "ps3build" | "ps3compact" => "makeps3iso",
        "ps3split" => "splitps3iso",
        _ => "extractps3iso",
    }
}

pub fn is_build_mode(mode: &str) -> bool {
    matches!(mode, "ps3build" | "ps3compact")
}

/// Inventario estable para comprobar que reconstruir un ISO no ha perdido ni
/// alterado el tamano de ningun archivo.
pub fn manifest(dir: &Path) -> anyhow::Result<Vec<(String, u64)>> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, u64)>) -> anyhow::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.metadata()?;
            if meta.is_dir() {
                walk(base, &path, out)?;
            } else if meta.is_file() {
                let relative = path
                    .strip_prefix(base)?
                    .to_string_lossy()
                    .replace('\\', "/")
                    .to_lowercase();
                out.push((relative, meta.len()));
            }
        }
        Ok(())
    }

    let mut files = vec![];
    walk(dir, dir, &mut files)?;
    files.sort_unstable();
    Ok(files)
}

/// `makeps3iso` se compila con una exclusion fija y sensible a mayusculas para
/// `PS3_UPDATE`. Durante la construccion se cambia solo el uso de mayusculas
/// para que la utilidad conserve esa carpeta; al escribir el ISO vuelve a
/// normalizar el nombre a mayusculas.
pub struct UpdateCaseGuard {
    original: PathBuf,
    temporary: PathBuf,
    adjusted: Option<PathBuf>,
}

impl UpdateCaseGuard {
    pub fn restore(&mut self) -> anyhow::Result<()> {
        let Some(adjusted) = self.adjusted.take() else {
            return Ok(());
        };
        std::fs::rename(&adjusted, &self.temporary)?;
        std::fs::rename(&self.temporary, &self.original)?;
        Ok(())
    }
}

impl Drop for UpdateCaseGuard {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

pub fn preserve_update_for_builder(dir: &Path) -> anyhow::Result<Option<UpdateCaseGuard>> {
    let Some(update) = std::fs::read_dir(dir)?
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("PS3_UPDATE")
        })
        .map(|entry| entry.path())
    else {
        return Ok(None);
    };
    if update
        .file_name()
        .map(|name| name.to_string_lossy() != "PS3_UPDATE")
        .unwrap_or(true)
    {
        return Ok(None);
    }

    let temporary = dir.join(".chd-studio-update-case");
    let adjusted = dir.join("ps3_update");
    let lowercase_conflict = std::fs::read_dir(dir)?
        .flatten()
        .any(|entry| entry.file_name().to_string_lossy() == "ps3_update" && entry.path() != update);
    anyhow::ensure!(
        !temporary.exists() && !lowercase_conflict,
        "No se puede preservar PS3_UPDATE porque hay un nombre temporal en conflicto"
    );
    std::fs::rename(&update, &temporary)?;
    if let Err(error) = std::fs::rename(&temporary, &adjusted) {
        let _ = std::fs::rename(&temporary, &update);
        return Err(error.into());
    }
    Ok(Some(UpdateCaseGuard {
        original: update,
        temporary,
        adjusted: Some(adjusted),
    }))
}

/// Tamano fisico ocupado tras aplicar compresion transparente. En Unix `blocks`
/// ya expresa los bloques realmente asignados; en Windows se consulta la API que
/// entiende archivos NTFS comprimidos.
pub fn allocated_size(path: &Path) -> u64 {
    if path.is_dir() {
        return std::fs::read_dir(path)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|entry| allocated_size(&entry.path()))
                    .sum()
            })
            .unwrap_or(0);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        return std::fs::metadata(path)
            .map(|meta| meta.blocks() * 512)
            .unwrap_or(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        type Dword = u32;
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCompressedFileSizeW(file_name: *const u16, high: *mut Dword) -> Dword;
        }
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut high = 0u32;
        // SAFETY: `wide` termina en NUL y `high` apunta a memoria valida.
        let low = unsafe { GetCompressedFileSizeW(wide.as_ptr(), &mut high) };
        if low == u32::MAX && high == 0 {
            return std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
        }
        return ((high as u64) << 32) | low as u64;
    }

    #[cfg(not(any(unix, windows)))]
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

/// `extractps3iso [-s] <ISO> <carpeta destino>`
pub fn extract_args(input: &str, dest: &str, split: bool) -> Vec<String> {
    let mut a = vec![];
    if split {
        a.push("-s".into());
    }
    a.push(input.to_string());
    a.push(dest.to_string());
    a
}

/// `makeps3iso [-s] <carpeta> <ISO destino>`
pub fn build_args(dir: &str, output: &str, split: bool) -> Vec<String> {
    let mut a = vec![];
    if split {
        a.push("-s".into());
    }
    a.push(dir.to_string());
    a.push(output.to_string());
    a
}

/// `splitps3iso <ISO>`
pub fn split_args(input: &str) -> Vec<String> {
    vec![input.to_string()]
}

// ------------------------------------------------------------ heuristicas

/// Archivos y carpetas sin los cuales el juego no arranca. Nunca se proponen
/// para borrar, y `trim` se niega a tocarlos aunque se los pasen.
const PROTEGIDOS: &[&str] = &[
    "ps3_disc.sfb",
    "param.sfo",
    "eboot.bin",
    "ps3_game/licdir",
    "ps3_game/param.sfo",
    "ps3_game/usrdir/eboot.bin",
];

/// Codigos de idioma tal y como suelen aparecer en los nombres de archivo.
const IDIOMAS: &[(&str, &[&str])] = &[
    ("Inglés", &["_en", "eng", "english", "_us", "_uk"]),
    (
        "Español",
        &["_es", "spa", "spanish", "espanol", "castellano"],
    ),
    ("Francés", &["_fr", "fre", "fra", "french", "francais"]),
    ("Alemán", &["_de", "ger", "deu", "german", "deutsch"]),
    ("Italiano", &["_it", "ita", "italian", "italiano"]),
    ("Portugués", &["_pt", "por", "portuguese", "brazil"]),
    ("Japonés", &["_jp", "jpn", "japanese"]),
    ("Ruso", &["_ru", "rus", "russian"]),
    ("Coreano", &["_kr", "kor", "korean"]),
    ("Chino", &["_cn", "chi", "chinese", "_zh"]),
    ("Polaco", &["_pl", "pol", "polish"]),
    ("Neerlandés", &["_nl", "dut", "nld", "dutch"]),
];

/// Extensiones de video y audio, que es donde se va casi todo el espacio.
const MEDIA_EXT: &[&str] = &[
    "pam", "bik", "at3", "msf", "sgd", "mp4", "m2v", "wav", "ogg", "wmv",
];

#[derive(Debug, Clone, Serialize)]
pub struct Ps3Entry {
    /// Ruta relativa a la carpeta del juego
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    /// update | lang | media | core
    pub kind: String,
    /// Idioma detectado, si lo hay
    pub lang: Option<String>,
    /// true si CHD Studio lo propone para borrar
    pub suggested: bool,
    /// true si esta protegido y no se puede borrar
    pub protected: bool,
    /// Explicacion corta para la interfaz
    pub note: Option<String>,
}

fn es_protegido(rel: &str) -> bool {
    let r = rel.replace('\\', "/").to_lowercase();
    PROTEGIDOS
        .iter()
        .any(|p| r == *p || r.ends_with(&format!("/{p}")))
}

/// Intenta reconocer a que idioma pertenece un archivo por su nombre.
fn detectar_idioma(nombre: &str) -> Option<&'static str> {
    let n = nombre.to_lowercase();
    // Se busca el codigo pegado a un separador para no confundir "spain" con
    // el "_es" de, por ejemplo, "files.dat"
    for (idioma, marcas) in IDIOMAS {
        for m in *marcas {
            let con_sep = format!("_{}", m.trim_start_matches('_'));
            if n.contains(&con_sep) || n.starts_with(m.trim_start_matches('_')) {
                return Some(idioma);
            }
        }
    }
    None
}

fn clasificar(
    rel: &str,
    nombre: &str,
    is_dir: bool,
) -> (String, Option<String>, bool, Option<String>) {
    let r = rel.replace('\\', "/").to_lowercase();

    if r == "ps3_update" || r.starts_with("ps3_update/") {
        return (
            "update".into(),
            None,
            true,
            Some("Actualizador de firmware del disco. Solo hace falta para instalar el sistema desde el juego.".into()),
        );
    }

    if let Some(idioma) = detectar_idioma(nombre) {
        return (
            "lang".into(),
            Some(idioma.to_string()),
            false,
            Some(format!("Parece contenido en {idioma}")),
        );
    }

    let ext = Path::new(nombre)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !is_dir && MEDIA_EXT.contains(&ext.as_str()) {
        return ("media".into(), None, false, Some("Vídeo o audio".into()));
    }

    ("core".into(), None, false, None)
}

fn tam_carpeta(p: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(p) else {
        return 0;
    };
    rd.flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => tam_carpeta(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

fn recorrer(base: &Path, dir: &Path, out: &mut Vec<Ps3Entry>, depth: usize) {
    if depth > 4 || out.len() > 3000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };

    for e in rd.flatten() {
        let p = e.path();
        let Ok(rel) = p.strip_prefix(base) else {
            continue;
        };
        let rel_s = rel.to_string_lossy().to_string();
        let nombre = e.file_name().to_string_lossy().to_string();
        let is_dir = p.is_dir();
        let size = if is_dir {
            tam_carpeta(&p)
        } else {
            e.metadata().map(|m| m.len()).unwrap_or(0)
        };

        let (kind, lang, suggested, note) = clasificar(&rel_s, &nombre, is_dir);
        let protected = es_protegido(&rel_s);

        out.push(Ps3Entry {
            path: rel_s.clone(),
            name: nombre,
            size,
            is_dir,
            kind: kind.clone(),
            lang,
            suggested: suggested && !protected,
            protected,
            note,
        });

        // No se entra dentro de PS3_UPDATE: se trata como un bloque
        if is_dir && kind != "update" {
            recorrer(base, &p, out, depth + 1);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Ps3Scan {
    pub dir: String,
    pub total: u64,
    /// true si la carpeta tiene pinta de ser un juego de PS3 extraido
    pub valid: bool,
    pub title: Option<String>,
    pub entries: Vec<Ps3Entry>,
}

/// Comprueba que la carpeta sea de verdad un juego extraido antes de tocar nada.
fn parece_juego(dir: &Path) -> bool {
    dir.join("PS3_GAME").is_dir()
        || dir.join("ps3_game").is_dir()
        || dir.join("PS3_DISC.SFB").is_file()
        || dir.join("PS3_DISC.sfb").is_file()
}

/// Lee el nombre del juego del PARAM.SFO, que va en texto plano dentro.
fn leer_titulo(dir: &Path) -> Option<String> {
    let sfo = ["PS3_GAME/PARAM.SFO", "ps3_game/param.sfo"]
        .iter()
        .map(|p| dir.join(p))
        .find(|p| p.is_file())?;
    let datos = std::fs::read(sfo).ok()?;
    let texto = String::from_utf8_lossy(&datos);
    // El titulo aparece como una cadena legible larga entre los datos
    texto
        .split(|c: char| c.is_control())
        .filter(|s| s.len() > 3 && s.chars().all(|c| c.is_ascii_graphic() || c == ' '))
        .filter(|s| !s.contains("PARAM") && !s.contains("APP_VER") && !s.contains("TITLE_ID"))
        .max_by_key(|s| s.len())
        .map(|s| s.trim().to_string())
}

pub fn scan(dir: &str) -> Ps3Scan {
    let base = PathBuf::from(dir);
    let mut entries = vec![];

    if base.is_dir() {
        recorrer(&base, &base, &mut entries, 0);
    }
    entries.sort_by(|a, b| b.size.cmp(&a.size));

    Ps3Scan {
        dir: dir.to_string(),
        total: tam_carpeta(&base),
        valid: parece_juego(&base),
        title: leer_titulo(&base),
        entries,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrimResult {
    pub freed: u64,
    pub removed: usize,
    pub skipped: Vec<String>,
}

/// Borra lo seleccionado, saltandose lo protegido y cualquier ruta que intente
/// salirse de la carpeta del juego.
pub fn trim(dir: &str, paths: &[String]) -> anyhow::Result<TrimResult> {
    let base = std::fs::canonicalize(dir)?;
    anyhow::ensure!(
        parece_juego(&base),
        "Esa carpeta no parece un juego de PS3 extraido"
    );

    let mut freed = 0u64;
    let mut removed = 0usize;
    let mut skipped = vec![];

    for rel in paths {
        if es_protegido(rel) {
            skipped.push(format!("{rel} (protegido)"));
            continue;
        }

        let target = base.join(rel);
        // Que nadie se escape de la carpeta con ../
        let Ok(real) = std::fs::canonicalize(&target) else {
            skipped.push(format!("{rel} (no existe)"));
            continue;
        };
        if !real.starts_with(&base) {
            skipped.push(format!("{rel} (fuera de la carpeta)"));
            continue;
        }

        let size = if real.is_dir() {
            tam_carpeta(&real)
        } else {
            std::fs::metadata(&real).map(|m| m.len()).unwrap_or(0)
        };

        let res = if real.is_dir() {
            std::fs::remove_dir_all(&real)
        } else {
            std::fs::remove_file(&real)
        };

        match res {
            Ok(_) => {
                freed += size;
                removed += 1;
            }
            Err(e) => skipped.push(format!("{rel} ({e})")),
        }
    }

    Ok(TrimResult {
        freed,
        removed,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("chd-studio-ps3-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn manifest_is_sorted_and_compares_paths_case_insensitively() {
        let dir = test_dir("manifest");
        std::fs::create_dir_all(dir.join("PS3_GAME/USRDIR")).unwrap();
        std::fs::write(dir.join("PS3_GAME/USRDIR/Z.BIN"), b"1234").unwrap();
        std::fs::write(dir.join("PS3_GAME/PARAM.SFO"), b"123").unwrap();

        let files = manifest(&dir).unwrap();
        assert_eq!(
            files,
            vec![
                ("ps3_game/param.sfo".into(), 3),
                ("ps3_game/usrdir/z.bin".into(), 4),
            ]
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn compact_mode_is_treated_as_an_iso_build() {
        assert!(is_build_mode("ps3compact"));
        assert!(is_build_mode("ps3build"));
        assert!(!is_build_mode("ps3rpcs3"));
    }

    #[test]
    fn update_case_guard_restores_the_original_name() {
        let dir = test_dir("update-case");
        std::fs::create_dir(dir.join("PS3_UPDATE")).unwrap();
        {
            let mut guard = preserve_update_for_builder(&dir).unwrap().unwrap();
            assert!(dir.join("ps3_update").is_dir());
            guard.restore().unwrap();
        }
        assert!(dir.join("PS3_UPDATE").is_dir());
        let names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["PS3_UPDATE"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
