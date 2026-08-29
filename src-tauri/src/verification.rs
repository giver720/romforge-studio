//! Comprobacion comun de los resultados producidos por las distintas herramientas.
//!
//! Hay dos niveles deliberadamente distintos:
//!   * `passed`: la herramienta del formato comprobo internamente el resultado;
//!   * `basic`: CHD Studio comprobo existencia, tamano y estructura minima.
//!
//! Nunca se presenta una comprobacion estructural como si fuera una verificacion
//! criptografica. Los dos estados se muestran de forma separada en la interfaz.

use crate::chdman;
use crate::jobs::Job;
use crate::settings::Settings;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

#[derive(Clone, Debug)]
pub struct Outcome {
    pub status: &'static str,
    pub message: String,
}

#[derive(Debug)]
pub enum VerifyError {
    Failed(String),
    Canceled,
}

impl From<String> for VerifyError {
    fn from(value: String) -> Self {
        Self::Failed(value)
    }
}

impl Outcome {
    fn full(message: impl Into<String>) -> Self {
        Self {
            status: "passed",
            message: message.into(),
        }
    }

    fn basic(message: impl Into<String>) -> Self {
        Self {
            status: "basic",
            message: message.into(),
        }
    }
}

fn is_directory_output(job: &Job) -> bool {
    job.tool == "iso2god" || job.mode == "ps3extract" || job.mode == "iso2folder"
}

fn size_recursive(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => size_recursive(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

fn contains_name(dir: &Path, expected: &str, depth: usize) -> bool {
    if depth > 8 {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry
            .file_name()
            .to_string_lossy()
            .eq_ignore_ascii_case(expected)
        {
            return true;
        }
        if path.is_dir() && contains_name(&path, expected, depth + 1) {
            return true;
        }
    }
    false
}

fn check_chd_magic(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("No se pudo abrir el CHD: {e}"))?;
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic)
        .map_err(|_| "El CHD es demasiado pequeno para tener una cabecera valida".to_string())?;
    if &magic != b"MComprHD" {
        return Err("El archivo generado no tiene una cabecera CHD valida".into());
    }
    Ok(())
}

fn check_psp_magic(path: &Path, ext: &str) -> Result<(), String> {
    let expected: &[u8; 4] = match ext {
        "cso" => b"CISO",
        "zso" => b"ZISO",
        "dax" => b"DAX\0",
        _ => return Ok(()),
    };
    let mut file = File::open(path).map_err(|e| format!("No se pudo abrir el resultado: {e}"))?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)
        .map_err(|_| "El resultado es demasiado pequeno para contener una cabecera".to_string())?;
    if &magic != expected {
        return Err(format!("La cabecera no corresponde a un archivo .{ext}"));
    }
    Ok(())
}

fn split_parts(input: &Path) -> Vec<PathBuf> {
    let Some(parent) = input.parent() else {
        return vec![];
    };
    let Some(name) = input.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return vec![];
    };
    let prefix = format!("{name}.");
    let Ok(entries) = std::fs::read_dir(parent) else {
        return vec![];
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let Some(candidate) = path.file_name().map(|n| n.to_string_lossy()) else {
                return false;
            };
            candidate
                .strip_prefix(&prefix)
                .map(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
        })
        .collect()
}

/// Validacion que siempre se ejecuta, incluso cuando el formato no ofrece un
/// verificador independiente.
pub fn structural(job: &Job) -> Result<Outcome, String> {
    if matches!(job.mode.as_str(), "verify" | "wiiverify") {
        return Ok(Outcome::full(
            "La herramienta verifico el archivo sin errores",
        ));
    }

    if job.mode == "ps3split" {
        let input = Path::new(&job.input);
        let parts = split_parts(input);
        let input_size = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
        if !parts.is_empty() {
            if parts
                .iter()
                .any(|part| std::fs::metadata(part).map(|m| m.len()).unwrap_or(0) == 0)
            {
                return Err("La division produjo al menos un fragmento vacio".into());
            }
            return Ok(Outcome::basic(format!(
                "Se comprobaron {} fragmentos de PS3",
                parts.len()
            )));
        }
        if input_size <= 4 * 1024 * 1024 * 1024u64 {
            return Ok(Outcome::basic(
                "El ISO no necesitaba dividirse porque ya cabe en FAT32",
            ));
        }
        return Err("La herramienta termino sin crear los fragmentos esperados".into());
    }

    let output = Path::new(&job.output);
    if is_directory_output(job) {
        if !output.is_dir() {
            return Err("No aparecio la carpeta de salida esperada".into());
        }
        if size_recursive(output) == 0 {
            return Err("La carpeta de salida esta vacia".into());
        }
        if job.mode == "ps3extract" && !crate::ps3::scan(&job.output).valid {
            return Err("La carpeta extraida no contiene la estructura de un juego de PS3".into());
        }
        if job.mode == "iso2folder" && !contains_name(output, "default.xex", 0) {
            return Err("La extraccion termino sin encontrar default.xex".into());
        }
        return Ok(Outcome::basic("Carpeta y estructura minima comprobadas"));
    }

    // makeps3iso puede producir directamente game.iso.0, game.iso.1, etc.
    // cuando esta activa la compatibilidad FAT32; en ese caso no existe el
    // nombre base que figura como salida del trabajo.
    if job.mode == "ps3build" && !output.is_file() {
        let parts = split_parts(output);
        if parts.is_empty() {
            return Err("No aparecio el ISO ni sus fragmentos de salida".into());
        }
        if parts
            .iter()
            .any(|part| std::fs::metadata(part).map(|m| m.len()).unwrap_or(0) == 0)
        {
            return Err("La construccion produjo al menos un fragmento vacio".into());
        }
        return Ok(Outcome::basic(format!(
            "Se comprobaron {} fragmentos del ISO de PS3",
            parts.len()
        )));
    }

    // 4NXCI puede generar varios NSP cuyos nombres proceden de los title IDs,
    // no necesariamente del nombre del XCI. La salida logica del trabajo es
    // por eso la coleccion de NSP creada en su carpeta temporal.
    if job.tool == "4nxci" && !output.is_file() {
        let Some(parent) = output.parent() else {
            return Err("No se pudo localizar la carpeta de salida de 4NXCI".into());
        };
        let files: Vec<PathBuf> = std::fs::read_dir(parent)
            .map_err(|_| "No se pudo leer la carpeta de salida de 4NXCI".to_string())?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("nsp"))
                    .unwrap_or(false)
            })
            .collect();
        if files.is_empty() {
            return Err("4NXCI termino sin generar ningun NSP".into());
        }
        if files
            .iter()
            .any(|path| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) == 0)
        {
            return Err("4NXCI genero al menos un NSP vacio".into());
        }
        return Ok(Outcome::basic(format!(
            "Se comprobaron {} archivos NSP",
            files.len()
        )));
    }

    let meta = std::fs::metadata(output)
        .map_err(|_| "No aparecio el archivo de salida esperado".to_string())?;
    if !meta.is_file() || meta.len() == 0 {
        return Err("El archivo de salida esta vacio".into());
    }

    if let Some(extra) = &job.output_extra {
        let extra_meta = std::fs::metadata(extra)
            .map_err(|_| "Falta el archivo de pistas que acompana al descriptor".to_string())?;
        if !extra_meta.is_file() || extra_meta.len() == 0 {
            return Err("El archivo de pistas generado esta vacio".into());
        }
    }

    let ext = output
        .extension()
        .map(|value| value.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if ext == "chd" {
        check_chd_magic(output)?;
    }
    check_psp_magic(output, &ext)?;

    Ok(Outcome::basic(format!(
        "Archivo y cabecera comprobados ({} bytes)",
        meta.len()
    )))
}

fn last_meaningful_line(text: &str) -> &str {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("La herramienta rechazo el resultado")
}

/// Ejecuta la comprobacion mas fuerte que ofrece cada formato y cae de forma
/// explicita a la validacion estructural cuando no existe una herramienta de
/// verificacion independiente.
pub async fn verify(
    job: &Job,
    settings: &Settings,
    primary_exe: &Path,
    cancel: &AtomicBool,
) -> Result<Outcome, VerifyError> {
    let basic = structural(job).map_err(VerifyError::Failed)?;

    if job.tool == "chdman" && job.mode.starts_with("create") {
        let args = vec!["verify".to_string(), "-i".to_string(), job.output.clone()];
        return match chdman::run_capture_cancelable(primary_exe, &args, cancel).await {
            Ok(chdman::CaptureResult::Finished(true, _)) => {
                Ok(Outcome::full("CHD verificado completamente con chdman"))
            }
            Ok(chdman::CaptureResult::Finished(false, text)) => Err(VerifyError::Failed(format!(
                "chdman no pudo verificar el resultado: {}",
                last_meaningful_line(&text)
            ))),
            Ok(chdman::CaptureResult::Canceled) => Err(VerifyError::Canceled),
            Err(e) => Err(VerifyError::Failed(format!(
                "No se pudo ejecutar la verificacion de chdman: {e}"
            ))),
        };
    }

    if matches!(job.tool.as_str(), "dolphintool" | "wit")
        && matches!(
            job.mode.as_str(),
            "iso2rvz" | "iso2wia" | "iso2gcz" | "rvz2iso" | "iso2wbfs"
        )
    {
        let dolphin = if job.tool == "dolphintool" {
            Some(primary_exe.to_path_buf())
        } else {
            crate::tools::locate("dolphintool", settings).map(|(path, _)| path)
        };
        if let Some(dolphin) = dolphin {
            let user = crate::settings::config_dir().join("dolphin");
            let _ = std::fs::create_dir_all(&user);
            let args = crate::wii::verify_args(&job.output, &user.to_string_lossy());
            return match chdman::run_capture_cancelable(&dolphin, &args, cancel).await {
                Ok(chdman::CaptureResult::Finished(true, _)) => {
                    Ok(Outcome::full("Imagen verificada con DolphinTool"))
                }
                Ok(chdman::CaptureResult::Finished(false, text)) => {
                    Err(VerifyError::Failed(format!(
                        "DolphinTool no pudo verificar el resultado: {}",
                        last_meaningful_line(&text)
                    )))
                }
                Ok(chdman::CaptureResult::Canceled) => Err(VerifyError::Canceled),
                Err(e) => Err(VerifyError::Failed(format!(
                    "No se pudo iniciar DolphinTool para verificar: {e}"
                ))),
            };
        }
    }

    // NSZ valida hashes durante la propia compresion mediante -V. Si el
    // proceso principal termino bien, esa comprobacion tambien termino bien.
    if job.tool == "nsz" && matches!(job.mode.as_str(), "nsp2nsz" | "xci2xcz") {
        return Ok(Outcome::full("Hashes del resultado comprobados por NSZ"));
    }

    Ok(basic)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "chd-studio-verification-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn rejects_empty_outputs() {
        let dir = test_dir("empty");
        let output = dir.join("game.cso");
        File::create(&output).unwrap();
        let job = Job::new(
            dir.join("game.iso").to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
            "maxcso".into(),
            "iso2cso".into(),
            "psp".into(),
        );
        assert!(structural(&job).unwrap_err().contains("vacio"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn recognizes_chd_header() {
        let dir = test_dir("chd");
        let output = dir.join("game.chd");
        let mut file = File::create(&output).unwrap();
        file.write_all(b"MComprHDresto").unwrap();
        let job = Job::new(
            dir.join("game.cue").to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
            "chdman".into(),
            "createcd".into(),
            "psx".into(),
        );
        assert_eq!(structural(&job).unwrap().status, "basic");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn xbox_folder_requires_default_xex() {
        let dir = test_dir("xbox");
        let output = dir.join("game");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(output.join("data.bin"), b"data").unwrap();
        let job = Job::new(
            dir.join("game.iso").to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
            "xiso".into(),
            "iso2folder".into(),
            "xbox360".into(),
        );
        assert!(structural(&job).unwrap_err().contains("default.xex"));
        std::fs::write(output.join("default.xex"), b"xex").unwrap();
        assert_eq!(structural(&job).unwrap().status, "basic");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn accepts_nonempty_ps3_build_fragments() {
        let dir = test_dir("ps3-build-parts");
        let output = dir.join("game.iso");
        std::fs::write(dir.join("game.iso.0"), b"first").unwrap();
        std::fs::write(dir.join("game.iso.1"), b"second").unwrap();
        let job = Job::new(
            dir.join("PS3_GAME").to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
            "ps3iso".into(),
            "ps3build".into(),
            "ps3".into(),
        );
        let outcome = structural(&job).unwrap();
        assert_eq!(outcome.status, "basic");
        assert!(outcome.message.contains("2 fragmentos"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn accepts_title_named_4nxci_outputs() {
        let dir = test_dir("4nxci-title-output");
        std::fs::write(dir.join("0100DEADBEEF0000.nsp"), b"package").unwrap();
        let job = Job::new(
            dir.join("game.xci").to_string_lossy().to_string(),
            dir.join("game.nsp").to_string_lossy().to_string(),
            "4nxci".into(),
            "xci2nsp".into(),
            "switch".into(),
        );
        let outcome = structural(&job).unwrap();
        assert_eq!(outcome.status, "basic");
        assert!(outcome.message.contains("1 archivos NSP"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
