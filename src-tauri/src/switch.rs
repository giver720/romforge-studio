//! Conversiones de formatos de Nintendo Switch.
//!
//! Se apoya en dos herramientas externas:
//!   * `nsz`   - comprime y descomprime NSP<->NSZ y XCI<->XCZ (zstd).
//!   * `4nxci` - convierte cartuchos XCI a instalables NSP.
//!
//! Ambas necesitan que el usuario tenga sus propias `prod.keys`. CHD Studio
//! solo comprueba si el archivo existe donde las herramientas lo esperan.

use crate::settings::Settings;
use serde::Serialize;
use std::path::PathBuf;

/// Modos expuestos en la interfaz, con la herramienta que los resuelve.
pub const MODES: &[(&str, &str, &str, &str)] = &[
    // (modo, herramienta, extension de entrada, extension de salida)
    ("nsp2nsz", "nsz", "nsp", "nsz"),
    ("nsz2nsp", "nsz", "nsz", "nsp"),
    ("xci2xcz", "nsz", "xci", "xcz"),
    ("xcz2xci", "nsz", "xcz", "xci"),
    ("xci2nsp", "4nxci", "xci", "nsp"),
];

pub fn tool_for(mode: &str) -> Option<&'static str> {
    MODES.iter().find(|m| m.0 == mode).map(|m| m.1)
}

pub fn output_ext(mode: &str) -> Option<&'static str> {
    MODES.iter().find(|m| m.0 == mode).map(|m| m.3)
}

/// Modo sugerido a partir de la extension del archivo.
pub fn suggest_mode(ext: &str) -> Option<&'static str> {
    match ext {
        "nsp" => Some("nsp2nsz"),
        "nsz" => Some("nsz2nsp"),
        "xci" => Some("xci2xcz"),
        "xcz" => Some("xcz2xci"),
        _ => None,
    }
}

pub fn is_switch_ext(ext: &str) -> bool {
    matches!(ext, "nsp" | "nsz" | "xci" | "xcz")
}

/// Ruta a las prod.keys: primero la que haya elegido el usuario, y si no las
/// carpetas donde las herramientas las buscan por su cuenta.
pub fn keys_path(s: &Settings) -> Option<PathBuf> {
    if let Some(p) = &s.switch_keys_path {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
        // Tambien se admite señalar la carpeta que las contiene
        for name in ["prod.keys", "keys.txt"] {
            let f = p.join(name);
            if f.is_file() {
                return Some(f);
            }
        }
    }

    let home = dirs::home_dir()?;
    let candidates = [
        home.join(".switch").join("prod.keys"),
        home.join(".switch").join("keys.txt"),
        home.join("switch").join("prod.keys"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

#[derive(Debug, Clone, Serialize)]
pub struct KeysStatus {
    pub found: bool,
    pub path: Option<String>,
    /// Donde deberia colocarlas el usuario si no las tiene.
    pub expected: String,
    /// true si la ruta viene de los ajustes y no de la carpeta por defecto.
    pub custom: bool,
}

pub fn keys_status(s: &Settings) -> KeysStatus {
    let expected = dirs::home_dir()
        .map(|h| h.join(".switch").join("prod.keys"))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "%USERPROFILE%\\.switch\\prod.keys".into());

    let found = keys_path(s);
    let custom = found
        .as_ref()
        .zip(s.switch_keys_path.as_ref())
        .map(|(f, c)| f.starts_with(c))
        .unwrap_or(false);

    KeysStatus {
        found: found.is_some(),
        path: found.map(|p| p.to_string_lossy().to_string()),
        expected,
        custom,
    }
}

/// Argumentos de `nsz`. Trabaja siempre sobre una carpeta de salida (-o).
///
/// nsz conserva el original salvo que se le pase `--rm-source`, asi que el
/// borrado lo decide CHD Studio despues, igual que con el resto de motores.
pub fn nsz_args(mode: &str, input: &str, out_dir: &str, s: &Settings) -> Vec<String> {
    let mut a: Vec<String> = vec![];

    match mode {
        "nsp2nsz" | "xci2xcz" => {
            a.push("-C".into());
            a.push("-l".into());
            a.push(s.nsz_level.clamp(1, 22).to_string());
            // La verificacion forma parte de la conversion: un trabajo no se
            // marca como terminado hasta que NSZ comprueba sus hashes.
            a.push("-V".into());
        }
        _ => a.push("-D".into()),
    }

    if s.nsz_threads > 0 {
        a.push("-t".into());
        a.push(s.nsz_threads.to_string());
    }

    if s.overwrite {
        a.push("-w".into());
    }

    if let Some(k) = keys_path(s) {
        a.push("--keys".into());
        a.push(k.to_string_lossy().to_string());
    }

    a.push("-o".into());
    a.push(out_dir.to_string());
    a.push(input.to_string());
    a
}

/// Argumentos de `4nxci`. Vuelca los NSP resultantes en la carpeta indicada.
pub fn nxci_args(input: &str, out_dir: &str, s: &Settings) -> Vec<String> {
    let mut a = vec![
        "-c".into(),
        input.to_string(),
        "-o".into(),
        out_dir.to_string(),
    ];
    // 4nxci solo busca las claves en su propia carpeta, hay que señalárselas
    if let Some(k) = keys_path(s) {
        a.push("-k".into());
        a.push(k.to_string_lossy().to_string());
    }
    a
}
