use flate2::{write::ZlibEncoder, Compression};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MODE_EXFAT: &str = "ps5exfat";
pub const MODE_FFPKG: &str = "ps5ffpkg";
pub const MODE_FFPFSC: &str = "ps5ffpfsc";
pub const MODE_COMPRESS: &str = "ps5compress";
pub const MODE_EXTRACT: &str = "ps5extract";
pub const CLUSTER_SIZE: u64 = 64 * 1024;
const SAMPLE_LIMIT: u64 = 32 * 1024 * 1024;
const SAMPLE_PER_FILE: u64 = 2 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct Ps5Scan {
    pub valid: bool,
    pub title_id: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
    pub file_count: u64,
    pub directory_count: u64,
    pub raw_bytes: u64,
    pub image_bytes: u64,
    pub compressed_estimate_bytes: u64,
    pub estimated_savings_percent: f32,
    pub recommended_format: String,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct Stats {
    files: u64,
    directories: u64,
    raw_bytes: u64,
    allocated_bytes: u64,
    sample_bytes: u64,
    sample_compressed_bytes: u64,
}

pub fn is_mode(mode: &str) -> bool {
    matches!(
        mode,
        MODE_EXFAT | MODE_FFPKG | MODE_FFPFSC | MODE_COMPRESS | MODE_EXTRACT
    )
}

pub fn tool_for(mode: &str) -> Option<&'static str> {
    match mode {
        MODE_FFPKG => Some("ufs2tool"),
        MODE_EXFAT | MODE_FFPFSC | MODE_COMPRESS | MODE_EXTRACT => Some("mkpfs"),
        _ => None,
    }
}

pub fn tool_for_input(mode: &str, input: &str) -> Option<&'static str> {
    if mode == MODE_EXTRACT
        && Path::new(input)
            .extension()
            .map(|ext| ext.eq_ignore_ascii_case("ffpkg"))
            .unwrap_or(false)
    {
        return Some("ufs2tool");
    }
    tool_for(mode)
}

pub fn output_ext(mode: &str) -> Option<&'static str> {
    match mode {
        MODE_EXFAT => Some("exfat"),
        MODE_FFPKG => Some("ffpkg"),
        MODE_FFPFSC | MODE_COMPRESS => Some("ffpfsc"),
        _ => None,
    }
}

pub fn writes_directory(mode: &str) -> bool {
    mode == MODE_EXTRACT
}

fn align(value: u64, unit: u64) -> u64 {
    value.saturating_add(unit - 1) / unit * unit
}

fn collect_stats(root: &Path, current: &Path, stats: &mut Stats) -> Result<(), String> {
    let entries = std::fs::read_dir(current)
        .map_err(|e| format!("No se pudo leer {}: {e}", current.display()))?;
    let mut names = std::collections::HashSet::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("No se pudo leer una entrada: {e}"))?;
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            return Err(format!(
                "El nombre no es Unicode valido: {}",
                path.display()
            ));
        };
        if name.ends_with(['.', ' '])
            || name.chars().any(|c| {
                c < '\u{20}' || ['<', '>', ':', '"', '/', '\\', '|', '?', '*'].contains(&c)
            })
        {
            return Err(format!("Nombre incompatible con exFAT: {name}"));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(format!(
                "Dos entradas de {} solo se diferencian por mayusculas; exFAT no puede guardarlas",
                current.display()
            ));
        }
        let meta = std::fs::symlink_metadata(&path)
            .map_err(|e| format!("No se pudo examinar {}: {e}", path.display()))?;
        if meta.file_type().is_symlink() {
            return Err(format!(
                "El dump contiene un enlace simbolico, que no se puede guardar de forma segura: {}",
                path.strip_prefix(root).unwrap_or(&path).display()
            ));
        }
        if meta.is_dir() {
            stats.directories += 1;
            collect_stats(root, &path, stats)?;
        } else if meta.is_file() {
            stats.files += 1;
            stats.raw_bytes = stats.raw_bytes.saturating_add(meta.len());
            stats.allocated_bytes = stats
                .allocated_bytes
                .saturating_add(align(meta.len(), CLUSTER_SIZE));
            sample_file(&path, stats);
        }
    }
    Ok(())
}

fn sample_file(path: &Path, stats: &mut Stats) {
    if stats.sample_bytes >= SAMPLE_LIMIT {
        return;
    }
    let remaining = (SAMPLE_LIMIT - stats.sample_bytes).min(SAMPLE_PER_FILE);
    let Ok(file) = std::fs::File::open(path) else {
        return;
    };
    let mut bytes = Vec::with_capacity(remaining as usize);
    if file.take(remaining).read_to_end(&mut bytes).is_err() || bytes.is_empty() {
        return;
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(7));
    if encoder.write_all(&bytes).is_err() {
        return;
    }
    let Ok(compressed) = encoder.finish() else {
        return;
    };
    stats.sample_bytes = stats.sample_bytes.saturating_add(bytes.len() as u64);
    stats.sample_compressed_bytes = stats
        .sample_compressed_bytes
        .saturating_add(compressed.len() as u64);
}

fn compressed_estimate(stats: &Stats) -> u64 {
    if stats.sample_bytes == 0 {
        return stats.raw_bytes.max(512 * 1024);
    }
    let ratio = stats.sample_compressed_bytes as f64 / stats.sample_bytes as f64;
    let payload = (stats.raw_bytes as f64 * ratio.clamp(0.05, 1.0)) as u64;
    align(payload.saturating_add(payload / 50), CLUSTER_SIZE).max(512 * 1024)
}

fn image_size(stats: &Stats) -> u64 {
    // MkPFS escribe una imagen ajustada: cabeceras/FAT, bitmap, tabla up-case,
    // un cluster por directorio y los clusters reales de los archivos. Para
    // directorios excepcionalmente grandes reservamos clusters adicionales.
    let directory_clusters = stats
        .directories
        .saturating_add(1)
        .saturating_add((stats.files + stats.directories) / 512);
    let metadata_clusters = 2u64;
    align(
        128u64
            .saturating_mul(1024)
            .saturating_add(stats.allocated_bytes)
            .saturating_add(
                directory_clusters
                    .saturating_add(metadata_clusters)
                    .saturating_mul(CLUSTER_SIZE),
            ),
        CLUSTER_SIZE,
    )
}

fn find_json_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                if keys.iter().any(|wanted| key.eq_ignore_ascii_case(wanted)) {
                    if let Some(text) = value.as_str().filter(|s| !s.trim().is_empty()) {
                        return Some(text.trim().to_string());
                    }
                }
            }
            map.values().find_map(|v| find_json_string(v, keys))
        }
        serde_json::Value::Array(values) => values.iter().find_map(|v| find_json_string(v, keys)),
        _ => None,
    }
}

fn metadata(root: &Path) -> Result<(Option<String>, Option<String>, Option<String>), String> {
    let path = root.join("sce_sys").join("param.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer sce_sys/param.json: {e}"))?;
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|e| format!("sce_sys/param.json no es JSON valido: {e}"))?;
    Ok((
        find_json_string(&value, &["titleId", "title_id"]),
        find_json_string(&value, &["titleName", "title_name", "name"]),
        find_json_string(&value, &["contentVersion", "content_version", "version"]),
    ))
}

pub fn scan(dir: &str) -> Ps5Scan {
    let root = Path::new(dir);
    let invalid = |error: String| Ps5Scan {
        valid: false,
        title_id: None,
        title: None,
        version: None,
        file_count: 0,
        directory_count: 0,
        raw_bytes: 0,
        image_bytes: 0,
        compressed_estimate_bytes: 0,
        estimated_savings_percent: 0.0,
        recommended_format: "ffpkg".into(),
        warnings: vec![],
        error: Some(error),
    };
    if !root.is_dir() {
        return invalid("La carpeta seleccionada no existe".into());
    }
    if !root.join("eboot.bin").is_file() {
        return invalid("Falta eboot.bin en la raiz del juego".into());
    }
    if !root.join("sce_sys").join("param.json").is_file() {
        return invalid("Falta sce_sys/param.json en la raiz del juego".into());
    }
    let mut stats = Stats::default();
    if let Err(error) = collect_stats(root, root, &mut stats) {
        return invalid(error);
    }
    if stats.files == 0 {
        return invalid("La carpeta del juego esta vacia".into());
    }
    let (title_id, title, version) = match metadata(root) {
        Ok(value) => value,
        Err(error) => return invalid(error),
    };
    let compressed_estimate_bytes = compressed_estimate(&stats);
    let estimated_savings_percent = if stats.raw_bytes == 0 {
        0.0
    } else {
        (100.0 - compressed_estimate_bytes as f32 * 100.0 / stats.raw_bytes as f32).clamp(0.0, 95.0)
    };
    let mut warnings = vec![];
    if stats.files > 0 && stats.raw_bytes / stats.files < 128 * 1024 {
        warnings.push(
            "El dump contiene muchos archivos pequeños; la estimación de FFPFSC puede variar"
                .into(),
        );
    }
    if estimated_savings_percent < 10.0 {
        warnings.push("El muestreo indica que FFPFSC ahorraría poco espacio".into());
    }
    Ps5Scan {
        valid: true,
        title_id,
        title,
        version,
        file_count: stats.files,
        directory_count: stats.directories,
        raw_bytes: stats.raw_bytes,
        image_bytes: image_size(&stats),
        compressed_estimate_bytes,
        estimated_savings_percent,
        recommended_format: "ffpkg".into(),
        warnings,
        error: None,
    }
}

pub type Manifest = BTreeMap<PathBuf, u64>;

fn manifest_walk(root: &Path, current: &Path, out: &mut Manifest) -> Result<(), String> {
    for entry in std::fs::read_dir(current).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        if meta.is_dir() {
            manifest_walk(root, &path, out)?;
        } else if meta.is_file() {
            out.insert(
                path.strip_prefix(root).unwrap_or(&path).to_path_buf(),
                meta.len(),
            );
        }
    }
    Ok(())
}

pub fn manifest(root: &Path) -> Result<Manifest, String> {
    let mut result = Manifest::new();
    manifest_walk(root, root, &mut result)?;
    Ok(result)
}

pub fn compare_trees(left: &Path, right: &Path, cancel: &AtomicBool) -> Result<bool, String> {
    let left_manifest = manifest(left)?;
    if left_manifest != manifest(right)? {
        return Ok(false);
    }
    let mut left_buffer = vec![0u8; 8 * 1024 * 1024];
    let mut right_buffer = vec![0u8; 8 * 1024 * 1024];
    for relative in left_manifest.keys() {
        let mut a = std::fs::File::open(left.join(relative)).map_err(|e| e.to_string())?;
        let mut b = std::fs::File::open(right.join(relative)).map_err(|e| e.to_string())?;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("__canceled__".into());
            }
            let a_len = a.read(&mut left_buffer).map_err(|e| e.to_string())?;
            let b_len = b.read(&mut right_buffer).map_err(|e| e.to_string())?;
            if a_len != b_len || left_buffer[..a_len] != right_buffer[..b_len] {
                return Ok(false);
            }
            if a_len == 0 {
                break;
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_game_root_and_sizes_image() {
        let root = std::env::temp_dir().join(format!("romforge-studio-ps5-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::write(root.join("eboot.bin"), vec![0u8; 100]).unwrap();
        std::fs::write(
            root.join("sce_sys/param.json"),
            r#"{"titleId":"PPSA12345","titleName":"Prueba","contentVersion":"01.000.000"}"#,
        )
        .unwrap();
        let result = scan(&root.to_string_lossy());
        assert!(result.valid);
        assert_eq!(result.title_id.as_deref(), Some("PPSA12345"));
        assert_eq!(result.file_count, 2);
        assert!(result.image_bytes > result.raw_bytes);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_an_extra_parent_folder() {
        let root =
            std::env::temp_dir().join(format!("romforge-studio-ps5-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("PPSA00000/sce_sys")).unwrap();
        std::fs::write(root.join("PPSA00000/eboot.bin"), b"x").unwrap();
        assert!(!scan(&root.to_string_lossy()).valid);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_invalid_param_json() {
        let root =
            std::env::temp_dir().join(format!("romforge-studio-ps5-json-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::write(root.join("eboot.bin"), b"x").unwrap();
        std::fs::write(root.join("sce_sys/param.json"), b"not-json").unwrap();
        let result = scan(&root.to_string_lossy());
        assert!(!result.valid);
        assert!(result.error.unwrap().contains("JSON valido"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn maps_ps5_modes_to_their_engines_and_extensions() {
        assert_eq!(tool_for(MODE_EXFAT), Some("mkpfs"));
        assert_eq!(tool_for(MODE_FFPKG), Some("ufs2tool"));
        assert_eq!(output_ext(MODE_FFPFSC), Some("ffpfsc"));
        assert_eq!(tool_for_input(MODE_EXTRACT, "game.ffpkg"), Some("ufs2tool"));
        assert_eq!(tool_for_input(MODE_EXTRACT, "game.ffpfsc"), Some("mkpfs"));
    }

    #[test]
    fn tree_comparison_detects_same_size_corruption() {
        let base = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-compare-{}",
            std::process::id()
        ));
        let left = base.join("left");
        let right = base.join("right");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&left).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::write(left.join("same.bin"), b"abcd").unwrap();
        std::fs::write(right.join("same.bin"), b"abce").unwrap();
        let cancel = AtomicBool::new(false);
        assert!(!compare_trees(&left, &right, &cancel).unwrap());
        std::fs::write(right.join("same.bin"), b"abcd").unwrap();
        assert!(compare_trees(&left, &right, &cancel).unwrap());
        let _ = std::fs::remove_dir_all(&base);
    }
}
