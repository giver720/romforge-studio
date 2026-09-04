//! Conversiones de formatos de Nintendo 3DS.
//!
//! Tres herramientas, cada una con su papel:
//!   * `z3ds_compressor` - comprime al formato Z3DS (zstd seekable) que Azahar
//!     admite desde la version 2123. Solo comprime: la descompresion la hace el
//!     propio emulador al cargar la ROM.
//!   * `3dsconv`         - CCI/.3ds -> CIA instalable. Necesita `boot9.bin`.
//!   * `cia-to-cci`      - CIA -> CCI descifrado. Necesita `aes_keys.txt`.
//!
//! Las claves son del usuario. ROMForge Studio comprueba si existen y avisa, pero no
//! las incluye ni ayuda a conseguirlas.

use crate::settings::Settings;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// (modo, herramienta, extension de salida)
pub const MODES: &[(&str, &str, &str)] = &[
    ("z3dscompress", "z3ds", ""), // la extension depende de la entrada
    ("cci2cia", "3dsconv", "cia"),
    // CIA -> CCI encadena ctrtool y makerom; se anota el primero
    ("cia2cci", "ctrtool", "cci"),
];

pub fn tool_for(mode: &str) -> Option<&'static str> {
    MODES
        .iter()
        .find(|m| m.0 == mode)
        .map(|m| m.1)
        .filter(|t| !t.is_empty())
}

pub fn is_mode(mode: &str) -> bool {
    MODES.iter().any(|m| m.0 == mode)
}

pub fn is_3ds_ext(ext: &str) -> bool {
    matches!(ext, "3ds" | "cci" | "cia" | "cxi" | "3dsx")
}

/// Extension comprimida que corresponde a cada formato de entrada.
/// `.3ds` es el mismo contenedor que `.cci`, asi que comparte destino.
pub fn z3ds_ext(ext: &str) -> &'static str {
    match ext {
        "cia" => "zcia",
        "cxi" => "zcxi",
        "3dsx" => "z3dsx",
        _ => "zcci",
    }
}

/// Extension de salida de un modo, sabiendo la de entrada.
pub fn output_ext(mode: &str, input_ext: &str) -> Option<&'static str> {
    match mode {
        "z3dscompress" => Some(z3ds_ext(input_ext)),
        "cci2cia" => Some("cia"),
        "cia2cci" => Some("cci"),
        _ => None,
    }
}

pub fn suggest_mode(ext: &str) -> &'static str {
    match ext {
        "cia" => "cia2cci",
        _ => "z3dscompress",
    }
}

// ------------------------------------------------------------------ claves

#[derive(Debug, Clone, Serialize)]
pub struct KeysStatus {
    /// boot9.bin, la bootROM de la consola
    pub boot9: Option<String>,
    /// aes_keys.txt, para herramientas antiguas
    pub aes_keys: Option<String>,
    /// seeddb.bin, imprescindible para los juegos con cifrado por semilla
    pub seeddb: Option<String>,
    pub expected_dir: String,
}

fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.is_file()).cloned()
}

/// Resuelve un archivo de claves: la ruta elegida a mano (archivo o carpeta que
/// lo contiene) tiene prioridad sobre la carpeta por defecto.
fn resolve(custom: &Option<String>, names: &[&str], fallback_dir: &PathBuf) -> Option<PathBuf> {
    if let Some(c) = custom {
        let p = PathBuf::from(c);
        if p.is_file() {
            return Some(p);
        }
        if let Some(found) = first_existing(&names.iter().map(|n| p.join(n)).collect::<Vec<_>>()) {
            return Some(found);
        }
    }
    first_existing(&names.iter().map(|n| fallback_dir.join(n)).collect::<Vec<_>>())
}

pub fn boot9_path(s: &Settings) -> Option<PathBuf> {
    let d = dirs::home_dir().unwrap_or_default().join(".3ds");
    resolve(&s.boot9_path, &["boot9.bin", "boot9_prot.bin"], &d)
}

pub fn aes_keys_path(s: &Settings) -> Option<PathBuf> {
    let d = dirs::home_dir().unwrap_or_default().join(".3ds");
    resolve(&s.aes_keys_path, &["aes_keys.txt"], &d)
}

pub fn keys_status(s: &Settings) -> KeysStatus {
    let d = dirs::home_dir().unwrap_or_default().join(".3ds");
    KeysStatus {
        boot9: boot9_path(s).map(|p| p.to_string_lossy().to_string()),
        aes_keys: aes_keys_path(s).map(|p| p.to_string_lossy().to_string()),
        seeddb: seeddb_path(s).map(|p| p.to_string_lossy().to_string()),
        expected_dir: d.to_string_lossy().to_string(),
    }
}

// ------------------------------------------------------------- argumentos

/// `z3ds_compressor <entrada> [salida]`. Le pasamos siempre la salida explicita
/// para que un `.3ds` acabe como `.zcci` y no como un `.z3ds` que Azahar ignora.
pub fn z3ds_args(input: &str, output: &str) -> Vec<String> {
    vec![input.to_string(), output.to_string()]
}

/// `3dsconv --output=<dir> [--overwrite] [--boot9=<ruta>] <entrada>`
pub fn conv_args(input: &str, out_dir: &str, s: &Settings) -> Vec<String> {
    let mut a = vec![format!("--output={out_dir}")];
    if s.overwrite {
        a.push("--overwrite".into());
    }
    if let Some(b9) = boot9_path(s) {
        a.push(format!("--boot9={}", b9.display()));
    }
    a.push(input.to_string());
    a
}

// ------------------------------------------------------- CIA -> CCI

/// Prefijo con el que ctrtool nombra el contenido que extrae.
pub const PREFIJO: &str = "c";

/// Las secciones en que se parte un NCCH. `arg` es como las llama ctrtool y
/// `flag` como las llama 3dstool al reconstruir.
pub const SECCIONES: &[(&str, &str, &str)] = &[
    // (nombre de archivo, opcion de ctrtool, opcion de 3dstool)
    ("exh.bin", "--exheader", "--exh"),
    ("logo.bin", "--logo", "--logo"),
    ("plain.bin", "--plainrgn", "--plain"),
    ("exefs.bin", "--exefs", "--exefs"),
    ("romfs.bin", "--romfs", "--romfs"),
];

pub fn seeddb_path(s: &Settings) -> Option<PathBuf> {
    let d = dirs::home_dir().unwrap_or_default().join(".3ds");
    resolve(&s.seeddb_path, &["seeddb.bin"], &d)
}

/// `3dstool -xvtf <tipo> <ncch> --header <salida>`
///
/// La cabecera NCCH va en claro, asi que se puede sacar del contenido cifrado.
/// ctrtool no la exporta, de ahi que haga falta 3dstool tambien para esto.
pub fn header_args(tipo: &str, ncch: &str, header: &str) -> Vec<String> {
    vec![
        "-xvtf".into(),
        tipo.to_string(),
        ncch.to_string(),
        "--header".into(),
        header.to_string(),
    ]
}

/// `ctrtool [--seeddb=...] --exheader=... --exefs=... --romfs=... <ncch>`
///
/// Aqui es donde ocurre el descifrado de verdad: ctrtool usa boot9 y, para los
/// juegos con cifrado por semilla, tambien la seeddb.
pub fn split_args(ncch: &str, dir: &Path, s: &Settings) -> Vec<String> {
    let mut a = vec![];
    if let Some(sd) = seeddb_path(s) {
        a.push(format!("--seeddb={}", sd.display()));
    }
    for (nombre, opt, _) in SECCIONES {
        a.push(format!("{opt}={}", dir.join(nombre).display()));
    }
    a.push(ncch.to_string());
    a
}

/// `3dstool -cvtf <tipo> <salida> --not-encrypt --header ... --exh ... [...]`
///
/// Solo se le pasan las secciones que ctrtool llego a escribir: no todos los
/// contenidos tienen logo o region plana. `--not-encrypt` deja marcado en la
/// cabecera que el resultado va sin cifrar, que es lo que espera Azahar.
pub fn rebuild_args(tipo: &str, salida: &str, dir: &Path) -> Vec<String> {
    let mut a = vec![
        "-cvtf".to_string(),
        tipo.to_string(),
        salida.to_string(),
        "--not-encrypt".into(),
        "--header".into(),
        dir.join("ncch.bin").to_string_lossy().to_string(),
    ];
    for (nombre, _, flag) in SECCIONES {
        let f = dir.join(nombre);
        if f.is_file() && std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0) > 0 {
            a.push(flag.to_string());
            a.push(f.to_string_lossy().to_string());
        }
    }
    a
}

/// La particion 0 lleva el ejecutable del juego; las demas son datos.
pub fn tipo_particion(idx: u32) -> &'static str {
    if idx == 0 {
        "cxi"
    } else {
        "cfa"
    }
}

/// `ctrtool --contents=<prefijo> <entrada.cia>`
///
/// Ojo: `--contents` NO es una carpeta aunque lo parezca. ctrtool lo usa como
/// prefijo y escribe `<prefijo>.<indice>.<id>`, asi que si se le pasa una
/// carpeta los archivos acaban al lado de ella, no dentro.
pub fn ctrtool_args(input: &str, work_dir: &Path) -> Vec<String> {
    let prefijo = work_dir.join(PREFIJO);
    vec![
        format!("--contents={}", prefijo.display()),
        input.to_string(),
    ]
}

/// ctrtool no tiene bandera para las claves: siempre lee `boot9.bin` de
/// `<HOME>/.3ds/`. Para respetar la ruta que haya elegido el usuario le
/// preparamos un HOME temporal con una copia del archivo dentro.
pub fn prepare_ctrtool_home(s: &Settings, work: &PathBuf) -> Option<PathBuf> {
    let b9 = boot9_path(s)?;
    let home = work.join("home");
    let d = home.join(".3ds");
    std::fs::create_dir_all(&d).ok()?;
    std::fs::copy(&b9, d.join("boot9.bin")).ok()?;
    Some(home)
}

/// Recoge lo que dejo ctrtool: archivos `c.<indice>.<id>` en la carpeta de
/// trabajo. Devuelve solo el nombre, porque makerom se ejecuta con esa carpeta
/// como directorio actual (ver `makerom_args`).
pub fn collect_contents(work_dir: &Path) -> Vec<(String, u32)> {
    let Ok(rd) = std::fs::read_dir(work_dir) else {
        return vec![];
    };

    let mut encontrados: Vec<(String, u32)> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter_map(|p| {
            let nombre = p.file_name()?.to_str()?.to_string();
            let mut partes = nombre.split('.');
            // Solo lo que empiece por el prefijo que le dimos a ctrtool
            if partes.next()? != PREFIJO {
                return None;
            }
            let idx = u32::from_str_radix(partes.next()?, 16).ok()?;
            Some((nombre, idx))
        })
        .collect();

    encontrados.sort_by_key(|(_, idx)| *idx);
    encontrados
}

/// `makerom -f cci -o <salida> -content <archivo>:<indice>:<id> ...`
///
/// Dos cosas que la ayuda de makerom no cuenta bien:
///   * para CCI tambien hacen falta los tres campos, no solo `archivo:indice`;
///   * parte el argumento por los dos puntos, asi que una ruta absoluta de
///     Windows (`C:\...`) lo rompe. Por eso se pasan nombres relativos y el
///     proceso se lanza con la carpeta de trabajo como directorio actual.
pub fn makerom_args(output: &str, contents: &[(String, u32)]) -> Vec<String> {
    let mut a = vec!["-f".into(), "cci".into(), "-o".into(), output.to_string()];
    for (nombre, idx) in contents {
        a.push("-content".into());
        a.push(format!("{nombre}:{idx}:{idx}"));
    }
    a
}
