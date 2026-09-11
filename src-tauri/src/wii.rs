//! Conversion de discos de Wii y GameCube con `DolphinTool`.
//!
//! El formato que interesa es **RVZ**, el propio de Dolphin. Ahi el ahorro sale
//! de dos sitios que se suman:
//!
//!   * la compresion con zstd, y
//!   * el **scrub**: los discos de Wii van rellenos de datos basura para
//!     entorpecer la copia, y quitarlos adelgaza el juego muchisimo mas que la
//!     compresion por si sola.
//!
//! DolphinTool viene dentro de Dolphin, que no publica en GitHub, asi que se
//! busca en el sistema en vez de descargarse.

use crate::settings::Settings;

/// (modo, formato de DolphinTool, extension de salida)
pub const MODES: &[(&str, &str, &str)] = &[
    ("iso2rvz", "rvz", "rvz"),
    ("iso2wia", "wia", "wia"),
    ("iso2gcz", "gcz", "gcz"),
    ("rvz2iso", "iso", "iso"),
];

/// WBFS es el formato de los cargadores USB de la Wii de verdad. DolphinTool
/// sabe leerlo pero no escribirlo, asi que para crearlo hace falta `wit`.
pub const MODE_WBFS: &str = "iso2wbfs";

pub fn is_mode(mode: &str) -> bool {
    MODES.iter().any(|m| m.0 == mode) || mode == "wiiverify" || mode == MODE_WBFS
}

/// Herramienta que ejecuta cada modo.
pub fn tool_for(mode: &str) -> &'static str {
    if mode == MODE_WBFS {
        "wit"
    } else {
        "dolphintool"
    }
}

pub fn output_ext(mode: &str) -> Option<&'static str> {
    if mode == "wiiverify" {
        return Some("");
    }
    if mode == MODE_WBFS {
        return Some("wbfs");
    }
    MODES.iter().find(|m| m.0 == mode).map(|m| m.2)
}

fn formato(mode: &str) -> Option<&'static str> {
    MODES.iter().find(|m| m.0 == mode).map(|m| m.1)
}

pub fn is_wii_ext(ext: &str) -> bool {
    matches!(ext, "iso" | "rvz" | "wia" | "gcz" | "gcm" | "wbfs" | "ciso")
}

/// Un ISO se comprime; cualquier otro formato se abre.
pub fn suggest_mode(ext: &str) -> &'static str {
    if ext == "iso" || ext == "gcm" {
        "iso2rvz"
    } else {
        "rvz2iso"
    }
}

/// `wit COPY <entrada> --wbfs [--split] --dest <salida>`
///
/// Opciones sacadas de la tabla de argumentos del propio wit: `-B/--wbfs`
/// elige el formato de salida, `-d/--dest` el destino y `-z/--split` parte el
/// resultado para que quepa en FAT32.
pub fn wbfs_args(input: &str, output: &str, s: &Settings) -> Vec<String> {
    let mut a = vec!["COPY".to_string(), input.to_string(), "--wbfs".into()];
    if s.wii_wbfs_split {
        a.push("--split".into());
    }
    // --DEST en mayusculas crea la carpeta de destino si no existe
    a.push("--DEST".into());
    a.push(output.to_string());
    a
}

/// Metodos de compresion que admite DolphinTool.
pub fn compresion(mode: &str, s: &Settings) -> Option<(&'static str, u8)> {
    // GCZ e ISO no admiten eleccion de metodo
    if mode != "iso2rvz" && mode != "iso2wia" {
        return None;
    }
    let nivel = s.wii_level.clamp(1, 22);
    Some(("zstd", nivel))
}

/// `DolphinTool convert -i <entrada> -o <salida> -f <formato> [-s] [-c zstd -l N -b N] -u <temp>`
pub fn convert_args(
    mode: &str,
    input: &str,
    output: &str,
    user_dir: &str,
    s: &Settings,
) -> Vec<String> {
    let mut a = vec![
        "convert".to_string(),
        "-i".into(),
        input.to_string(),
        "-o".into(),
        output.to_string(),
    ];

    if let Some(f) = formato(mode) {
        a.push("-f".into());
        a.push(f.into());
    }

    // El scrub solo tiene sentido al comprimir, no al volver a ISO
    if s.wii_scrub && mode != "rvz2iso" {
        a.push("-s".into());
    }

    if let Some((metodo, nivel)) = compresion(mode, s) {
        a.push("-c".into());
        a.push(metodo.into());
        a.push("-l".into());
        a.push(nivel.to_string());
        a.push("-b".into());
        // 128 KiB es el bloque que recomienda Dolphin para RVZ
        a.push("131072".into());
    }

    // Sin esto DolphinTool se crea una carpeta de usuario donde le parece
    a.push("-u".into());
    a.push(user_dir.to_string());
    a
}

/// `DolphinTool verify -i <entrada> -u <temp>`
pub fn verify_args(input: &str, user_dir: &str) -> Vec<String> {
    vec![
        "verify".into(),
        "-i".into(),
        input.to_string(),
        "-u".into(),
        user_dir.to_string(),
    ]
}
