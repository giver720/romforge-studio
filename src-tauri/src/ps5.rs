use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MODE: &str = "ps5exfat";
pub const CLUSTER_SIZE: u64 = 64 * 1024;

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
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct Stats {
    files: u64,
    directories: u64,
    raw_bytes: u64,
    allocated_bytes: u64,
}

pub fn is_mode(mode: &str) -> bool {
    mode == MODE
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
        }
    }
    Ok(())
}

fn image_size(stats: &Stats) -> u64 {
    let clusters = align(stats.allocated_bytes, CLUSTER_SIZE) / CLUSTER_SIZE;
    let fat = clusters.saturating_mul(4);
    let bitmap = align(clusters, 8) / 8;
    let entries = (stats.files + stats.directories).saturating_mul(512);
    let base = stats
        .allocated_bytes
        .saturating_add(fat)
        .saturating_add(bitmap)
        .saturating_add(entries)
        .saturating_add(64 * 1024 * 1024);
    let spare = (base / 100).clamp(1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    align(
        base.saturating_add(spare)
            .max(stats.raw_bytes.saturating_add(1024 * 1024 * 1024)),
        1024 * 1024,
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

fn metadata(root: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let path = root.join("sce_sys").join("param.json");
    let Ok(text) = std::fs::read_to_string(path) else {
        return (None, None, None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (None, None, None);
    };
    (
        find_json_string(&value, &["titleId", "title_id"]),
        find_json_string(&value, &["titleName", "title_name", "name"]),
        find_json_string(&value, &["contentVersion", "content_version", "version"]),
    )
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
    let (title_id, title, version) = metadata(root);
    Ps5Scan {
        valid: true,
        title_id,
        title,
        version,
        file_count: stats.files,
        directory_count: stats.directories,
        raw_bytes: stats.raw_bytes,
        image_bytes: image_size(&stats),
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

fn copy_file(source: &Path, destination: &Path, cancel: &AtomicBool) -> Result<bool, String> {
    let mut input =
        std::fs::File::open(source).map_err(|e| format!("{}: {e}", source.display()))?;
    let mut output = std::fs::File::create(destination)
        .map_err(|e| format!("{}: {e}", destination.display()))?;
    let mut buffer = vec![0u8; 8 * 1024 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let count = input.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
    }
    output.flush().map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn copy_tree(source: &Path, destination: &Path, cancel: &AtomicBool) -> Result<bool, String> {
    for entry in std::fs::read_dir(source).map_err(|e| e.to_string())? {
        if cancel.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        let meta = std::fs::symlink_metadata(&from).map_err(|e| e.to_string())?;
        if meta.file_type().is_symlink() {
            return Err(format!(
                "No se admiten enlaces simbolicos: {}",
                from.display()
            ));
        }
        if meta.is_dir() {
            std::fs::create_dir(&to).map_err(|e| format!("{}: {e}", to.display()))?;
            if !copy_tree(&from, &to, cancel)? {
                return Ok(false);
            }
        } else if meta.is_file() {
            if !copy_file(&from, &to, cancel)? {
                return Ok(false);
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
}
