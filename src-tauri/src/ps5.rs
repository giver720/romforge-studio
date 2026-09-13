use flate2::{write::ZlibEncoder, Compression};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MODE_EXFAT: &str = "ps5exfat";
pub const MODE_FFPKG: &str = "ps5ffpkg";
pub const MODE_FFPFSC: &str = "ps5ffpfsc";
pub const MODE_COMPRESS: &str = "ps5compress";
pub const MODE_EXTRACT: &str = "ps5extract";
pub const MODE_VERIFY: &str = "ps5verify";
/// Modos recientes con motores públicos y verificación independiente.
pub const MODE_NATIVE_FPKG: &str = "ps5fpkg";
/// Extrae una imagen exFAT a un workspace temporal y crea un FPKG nativo.
pub const MODE_EXFAT_FPKG: &str = "ps5fpkgexfat";
pub const MODE_LZ4: &str = "ps5lz4";
pub const CLUSTER_SIZE: u64 = 64 * 1024;
const SAMPLE_LIMIT: u64 = 32 * 1024 * 1024;
const SAMPLE_PER_FILE: u64 = 2 * 1024 * 1024;
const GIB: u64 = 1024 * 1024 * 1024;
const PFS_MAGIC: i64 = 20_130_315;
const PFS_MODE_SIGNED: u16 = 0x1;
const PFS_MODE_64BIT_INODES: u16 = 0x2;
const PFS_MODE_ENCRYPTED: u16 = 0x4;
const INODE_MODE_FILE: u16 = 0x8000;
const INODE_FLAG_COMPRESSED: u32 = 0x1;

#[derive(Clone, Debug, Serialize)]
pub struct Ps5Scan {
    pub valid: bool,
    pub title_id: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
    pub content_id: Option<String>,
    pub file_count: u64,
    pub directory_count: u64,
    pub raw_bytes: u64,
    pub image_bytes: u64,
    pub compressed_estimate_bytes: u64,
    pub estimated_savings_percent: f32,
    pub recommended_format: String,
    pub fpkg_ready: bool,
    pub fpkg_module_count: u64,
    pub fpkg_blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FpkgReadiness {
    pub ready: bool,
    pub module_count: u64,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OutputSpace {
    pub available_bytes: u64,
    pub location: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImageSpacePreflight {
    pub input_bytes: u64,
    pub available_bytes: u64,
    pub location: String,
    pub required_bytes: BTreeMap<String, u64>,
}

/// Consulta el espacio en la unidad real que recibirá la salida. La carpeta
/// configurada puede no existir todavía, así que se prueba su primer ancestro
/// existente en vez de asumir que la unidad del sistema es el destino.
pub fn output_space(destination: &Path) -> Result<OutputSpace, String> {
    let probe = destination
        .ancestors()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| "No se encontró una unidad válida para la salida PS5".to_string())?;
    let available_bytes = fs2::available_space(probe)
        .map_err(|error| format!("No se pudo consultar el espacio libre: {error}"))?;
    Ok(OutputSpace {
        available_bytes,
        location: probe.to_string_lossy().to_string(),
    })
}

fn with_space_margin(payload: u64, minimum_margin: u64) -> u64 {
    payload.saturating_add((payload / 10).max(minimum_margin))
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, String> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .ok_or_else(|| "Cabecera PFS truncada".to_string())?
        .try_into()
        .map_err(|_| "Cabecera PFS inválida".to_string())?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(|| "Cabecera PFS truncada".to_string())?
        .try_into()
        .map_err(|_| "Cabecera PFS inválida".to_string())?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_i64(data: &[u8], offset: usize) -> Result<i64, String> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or_else(|| "Cabecera PFS truncada".to_string())?
        .try_into()
        .map_err(|_| "Cabecera PFS inválida".to_string())?;
    Ok(i64::from_le_bytes(bytes))
}

/// Suma los tamaños lógicos declarados por los inodos sin leer ni descomprimir
/// sus datos. Esto permite anticipar cuánto ocupará una extracción FFPFSC.
fn pfs_logical_file_bytes(image: &Path) -> Result<u64, String> {
    let mut file = std::fs::File::open(image)
        .map_err(|error| format!("No se pudo abrir la imagen PFS: {error}"))?;
    let file_bytes = file
        .metadata()
        .map_err(|error| format!("No se pudo leer el tamaño de la imagen: {error}"))?
        .len();
    let mut header = [0u8; 0x400];
    file.read_exact(&mut header)
        .map_err(|_| "La imagen PFS está truncada".to_string())?;

    let version = read_i64(&header, 0x00)?;
    let magic = read_i64(&header, 0x08)?;
    let mode = read_u16(&header, 0x1c)?;
    let block_size = u64::from(read_u32(&header, 0x20)?);
    let inode_count = read_i64(&header, 0x30)?;
    let inode_block_count = read_i64(&header, 0x40)?;
    if magic != PFS_MAGIC || !matches!(version, 1 | 2) {
        return Err("La imagen no contiene una cabecera PFS compatible".into());
    }
    if mode & PFS_MODE_ENCRYPTED != 0 {
        return Err("No se puede estimar una imagen PFS cifrada sin su clave EKPFS".into());
    }
    if !(4096..=16 * 1024 * 1024).contains(&block_size) || !block_size.is_power_of_two() {
        return Err("La imagen declara un tamaño de bloque PFS inválido".into());
    }
    let inode_count = u64::try_from(inode_count)
        .map_err(|_| "La imagen declara una cantidad de inodos inválida".to_string())?;
    let inode_block_count = u64::try_from(inode_block_count)
        .map_err(|_| "La tabla de inodos PFS es inválida".to_string())?;
    let inode_size = if mode & PFS_MODE_SIGNED == 0 {
        0xa8u64
    } else if mode & PFS_MODE_64BIT_INODES != 0 {
        0x310u64
    } else {
        0x2c8u64
    };
    let inodes_per_block = block_size / inode_size;
    if inodes_per_block == 0 {
        return Err("El bloque PFS es demasiado pequeño para sus inodos".into());
    }
    let expected_blocks = inode_count.saturating_add(inodes_per_block - 1) / inodes_per_block;
    if inode_block_count < expected_blocks {
        return Err("La tabla de inodos PFS está incompleta".into());
    }
    let table_end = block_size
        .checked_add(
            inode_block_count
                .checked_mul(block_size)
                .ok_or_else(|| "La tabla de inodos PFS es demasiado grande".to_string())?,
        )
        .ok_or_else(|| "La tabla de inodos PFS es demasiado grande".to_string())?;
    if table_end > file_bytes {
        return Err("La tabla de inodos PFS queda fuera de la imagen".into());
    }

    let mut fields = [0u8; 24];
    let mut total = 0u64;
    for index in 0..inode_count {
        let block = index / inodes_per_block;
        let slot = index % inodes_per_block;
        let offset = block_size
            .saturating_add(block.saturating_mul(block_size))
            .saturating_add(slot.saturating_mul(inode_size));
        file.seek(SeekFrom::Start(offset))
            .map_err(|error| format!("No se pudo leer la tabla de inodos PFS: {error}"))?;
        file.read_exact(&mut fields)
            .map_err(|_| "La tabla de inodos PFS está truncada".to_string())?;
        if read_u16(&fields, 0)? & INODE_MODE_FILE == 0 {
            continue;
        }
        let flags = read_u32(&fields, 4)?;
        let stored_or_logical = read_i64(&fields, 8)?;
        let compressed_logical = read_i64(&fields, 16)?;
        let logical = if flags & INODE_FLAG_COMPRESSED != 0 {
            compressed_logical
        } else {
            stored_or_logical
        };
        let logical = u64::try_from(logical)
            .map_err(|_| "Un inodo PFS declara un tamaño lógico inválido".to_string())?;
        total = total
            .checked_add(logical)
            .ok_or_else(|| "El tamaño lógico PFS supera el límite admitido".to_string())?;
    }
    Ok(total)
}

pub fn image_space_requirements(input: &Path) -> Result<BTreeMap<String, u64>, String> {
    if !input.is_file() {
        return Err("Selecciona un archivo de imagen PS5 válido".into());
    }
    let input_bytes = std::fs::metadata(input)
        .map_err(|error| format!("No se pudo leer la imagen: {error}"))?
        .len();
    let extension = input
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(extension.as_str(), "exfat" | "ffpkg" | "ffpfs" | "ffpfsc") {
        return Err("La extensión de la imagen PS5 no es compatible".into());
    }

    let mut requirements = BTreeMap::new();
    if matches!(extension.as_str(), "exfat" | "ffpkg") {
        requirements.insert(MODE_COMPRESS.into(), with_space_margin(input_bytes, GIB));
    }
    let extracted_bytes = if matches!(extension.as_str(), "ffpfs" | "ffpfsc") {
        pfs_logical_file_bytes(input)?
    } else {
        input_bytes
    };
    requirements.insert(MODE_EXTRACT.into(), with_space_margin(extracted_bytes, GIB));
    if extension == "exfat" {
        requirements.insert(
            MODE_EXFAT_FPKG.into(),
            input_bytes
                .saturating_mul(2)
                .saturating_add((input_bytes / 10).max(2 * GIB)),
        );
    }
    Ok(requirements)
}

pub fn image_required_space(input: &Path, mode: &str) -> Result<u64, String> {
    image_space_requirements(input)?
        .remove(mode)
        .ok_or_else(|| "La operación no es compatible con esta imagen PS5".to_string())
}

pub fn image_space_preflight(
    input: &Path,
    destination: &Path,
) -> Result<ImageSpacePreflight, String> {
    let input_bytes = std::fs::metadata(input)
        .map_err(|error| format!("No se pudo leer la imagen: {error}"))?
        .len();
    let required_bytes = image_space_requirements(input)?;
    let output = output_space(destination)?;
    Ok(ImageSpacePreflight {
        input_bytes,
        available_bytes: output.available_bytes,
        location: output.location,
        required_bytes,
    })
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
        MODE_EXFAT
            | MODE_FFPKG
            | MODE_FFPFSC
            | MODE_COMPRESS
            | MODE_EXTRACT
            | MODE_VERIFY
            | MODE_NATIVE_FPKG
            | MODE_EXFAT_FPKG
            | MODE_LZ4
    )
}

pub fn tool_for(mode: &str) -> Option<&'static str> {
    match mode {
        MODE_FFPKG => Some("ufs2tool"),
        MODE_NATIVE_FPKG | MODE_EXFAT_FPKG => Some("prospero"),
        MODE_LZ4 => Some("ampr"),
        MODE_EXFAT | MODE_FFPFSC | MODE_COMPRESS | MODE_EXTRACT | MODE_VERIFY => Some("mkpfs"),
        _ => None,
    }
}

pub fn tool_for_input(mode: &str, input: &str) -> Option<&'static str> {
    if matches!(mode, MODE_EXTRACT | MODE_VERIFY)
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
        MODE_NATIVE_FPKG | MODE_EXFAT_FPKG => Some("pkg"),
        MODE_FFPFSC | MODE_COMPRESS => Some("ffpfsc"),
        _ => None,
    }
}

pub fn writes_directory(mode: &str) -> bool {
    matches!(mode, MODE_EXTRACT | MODE_LZ4)
}

pub fn fpkg_convert_args(
    input: &str,
    output: &str,
    options: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    let decrypted_subfolder = options
        .get("decrypted_subfolder")
        .map(String::as_str)
        .unwrap_or("decrypted")
        .trim();
    validate_decrypted_subfolder(decrypted_subfolder)?;
    let embedded_right = match options.get("embedded_right").map(String::as_str) {
        None | Some("false") => "false",
        Some("true") => "true",
        Some(_) => return Err("La opción right.sprx integrado no es válida".into()),
    };
    Ok(vec![
        "convert".into(),
        "--input".into(),
        input.into(),
        "--output".into(),
        output.into(),
        "--decrypted-subfolder".into(),
        decrypted_subfolder.into(),
        "--embedded-right".into(),
        embedded_right.into(),
    ])
}

pub fn lz4_convert_args(
    input: &str,
    output: &str,
    options: &BTreeMap<String, String>,
) -> Result<Vec<String>, String> {
    let profile = options
        .get("profile")
        .map(String::as_str)
        .unwrap_or("balanced");
    if !matches!(profile, "fast" | "balanced" | "maximum") {
        return Err("El perfil AMPR/LZ4 no es válido".into());
    }
    Ok(vec![
        "convert".into(),
        "--input".into(),
        input.into(),
        "--output".into(),
        output.into(),
        "--profile".into(),
        profile.into(),
    ])
}

pub fn validate_decrypted_subfolder(value: &str) -> Result<&str, String> {
    let value = value.trim();
    let invalid_segment = value
        .split(['/', '\\'])
        .any(|segment| segment.is_empty() || matches!(segment, "." | ".."));
    if value.is_empty() || value.len() > 120 || invalid_segment || value.contains(':') {
        return Err(
            "La subcarpeta de módulos descifrados debe ser una ruta relativa segura".into(),
        );
    }
    Ok(value)
}

fn path_starts_with(candidate: &Path, base: &Path) -> bool {
    #[cfg(windows)]
    {
        let candidate: Vec<_> = candidate.components().collect();
        let base: Vec<_> = base.components().collect();
        candidate.len() >= base.len()
            && candidate.iter().zip(&base).all(|(left, right)| {
                left.as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
            })
    }
    #[cfg(not(windows))]
    {
        candidate.starts_with(base)
    }
}

/// Evita crear el staging o la salida dentro del dump que se está leyendo.
/// Para rutas que aún no existen se resuelve el primer ancestro existente, de
/// modo que también se detectan subcarpetas nuevas bajo el juego.
pub fn validate_output_location(input: &Path, output: &Path) -> Result<(), String> {
    if !input.is_dir() {
        return Ok(());
    }
    let input = std::fs::canonicalize(input)
        .map_err(|error| format!("No se pudo comprobar la carpeta de entrada: {error}"))?;
    let existing_output = output
        .ancestors()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| "No se pudo comprobar la ubicación de salida".to_string())?;
    let output = std::fs::canonicalize(existing_output)
        .map_err(|error| format!("No se pudo comprobar la carpeta de salida: {error}"))?;
    if path_starts_with(&output, &input) {
        return Err(
            "La salida PS5 no puede estar dentro del dump de origen. Elige una carpeta externa para evitar incluir la conversión dentro de sí misma."
                .into(),
        );
    }
    Ok(())
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

fn metadata(
    root: &Path,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
    String,
> {
    let path = root.join("sce_sys").join("param.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("No se pudo leer sce_sys/param.json: {e}"))?;
    let value = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|e| format!("sce_sys/param.json no es JSON valido: {e}"))?;
    Ok((
        find_json_string(&value, &["titleId", "title_id"]),
        find_json_string(&value, &["titleName", "title_name", "name"]),
        find_json_string(&value, &["contentVersion", "content_version", "version"]),
        find_json_string(&value, &["contentId", "content_id"]),
    ))
}

fn first_magic(path: &Path) -> Option<u32> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut bytes = [0u8; 4];
    file.read_exact(&mut bytes).ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn valid_content_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 36
        && bytes[6] == b'-'
        && bytes[16] == b'_'
        && bytes[19] == b'-'
        && bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 6 | 16 | 19) || byte.is_ascii_uppercase() || byte.is_ascii_digit()
        })
}

fn collect_fpkg_modules(
    root: &Path,
    current: &Path,
    decrypted_root: &Path,
    modules: &mut u64,
    blockers: &mut Vec<String>,
) {
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path == decrypted_root {
                continue;
            }
            collect_fpkg_modules(root, &path, decrypted_root, modules, blockers);
            continue;
        }
        let name = path
            .file_name()
            .map(|value| value.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if name != "eboot.bin"
            && !name.ends_with(".elf")
            && !name.ends_with(".prx")
            && !name.ends_with(".sprx")
        {
            continue;
        }
        *modules += 1;
        let relative = path.strip_prefix(root).unwrap_or(&path);
        match first_magic(&path) {
            // ELF sin cifrar: el motor puede firmarlo directamente.
            Some(0x464c_457f) => {}
            // SELF: hace falta su copia ELF bajo decrypted/ con la misma ruta.
            Some(0xeef5_1454) => {
                let decrypted = decrypted_root.join(relative);
                if first_magic(&decrypted) != Some(0x464c_457f) {
                    blockers.push(format!(
                        "Falta el ELF descifrado de {} en {}/{}",
                        relative.display(),
                        decrypted_root
                            .strip_prefix(root)
                            .unwrap_or(decrypted_root)
                            .display(),
                        relative.display()
                    ));
                }
            }
            _ => blockers.push(format!(
                "{} no es un módulo ELF/SELF reconocible",
                relative.display()
            )),
        }
    }
}

fn fpkg_readiness_for(
    root: &Path,
    content_id: Option<&str>,
    decrypted_subfolder: &str,
) -> Result<FpkgReadiness, String> {
    let decrypted_subfolder = validate_decrypted_subfolder(decrypted_subfolder)?;
    let decrypted_root = root.join(decrypted_subfolder);
    let mut blockers = vec![];
    let mut module_count = 0;
    collect_fpkg_modules(
        root,
        root,
        &decrypted_root,
        &mut module_count,
        &mut blockers,
    );
    if module_count == 0 {
        blockers.push("No se encontró ningún módulo ejecutable de PS5".into());
    }
    if content_id.map(valid_content_id) != Some(true) {
        blockers.push("sce_sys/param.json no contiene un contentId válido de 36 caracteres".into());
    }
    Ok(FpkgReadiness {
        ready: blockers.is_empty(),
        module_count,
        blockers,
    })
}

pub fn fpkg_readiness(dir: &str, decrypted_subfolder: &str) -> Result<FpkgReadiness, String> {
    let root = Path::new(dir);
    if !root.is_dir() {
        return Err("La carpeta seleccionada no existe".into());
    }
    let (_, _, _, content_id) = metadata(root)?;
    fpkg_readiness_for(root, content_id.as_deref(), decrypted_subfolder)
}

pub fn scan(dir: &str) -> Ps5Scan {
    scan_with_decrypted_subfolder(dir, "decrypted")
}

pub fn scan_with_decrypted_subfolder(dir: &str, decrypted_subfolder: &str) -> Ps5Scan {
    let root = Path::new(dir);
    let invalid = |error: String| Ps5Scan {
        valid: false,
        title_id: None,
        title: None,
        version: None,
        content_id: None,
        file_count: 0,
        directory_count: 0,
        raw_bytes: 0,
        image_bytes: 0,
        compressed_estimate_bytes: 0,
        estimated_savings_percent: 0.0,
        recommended_format: "ffpkg".into(),
        fpkg_ready: false,
        fpkg_module_count: 0,
        fpkg_blockers: vec![],
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
    let (title_id, title, version, content_id) = match metadata(root) {
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
    let readiness = fpkg_readiness_for(root, content_id.as_deref(), decrypted_subfolder)
        .unwrap_or_else(|error| FpkgReadiness {
            ready: false,
            module_count: 0,
            blockers: vec![error],
        });
    Ps5Scan {
        valid: true,
        title_id,
        title,
        version,
        content_id,
        file_count: stats.files,
        directory_count: stats.directories,
        raw_bytes: stats.raw_bytes,
        image_bytes: image_size(&stats),
        compressed_estimate_bytes,
        estimated_savings_percent,
        recommended_format: "ffpkg".into(),
        fpkg_ready: readiness.ready,
        fpkg_module_count: readiness.module_count,
        fpkg_blockers: readiness.blockers,
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
        assert!(!result.fpkg_ready);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_a_decrypted_dump_for_native_fpkg() {
        let root = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-fpkg-ready-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::write(root.join("eboot.bin"), [0x7f, b'E', b'L', b'F', 0, 0, 0, 0]).unwrap();
        std::fs::write(
            root.join("sce_sys/param.json"),
            r#"{"titleId":"PPSA99099","titleName":"Prueba","contentId":"UP9000-PPSA99099_00-PROSPERO00000000"}"#,
        )
        .unwrap();
        let result = scan(&root.to_string_lossy());
        assert!(result.valid);
        assert!(result.fpkg_ready, "{:?}", result.fpkg_blockers);
        assert_eq!(result.fpkg_module_count, 1);
        assert_eq!(
            result.content_id.as_deref(),
            Some("UP9000-PPSA99099_00-PROSPERO00000000")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn blocks_native_fpkg_when_a_self_has_no_decrypted_elf() {
        let root = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-fpkg-encrypted-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::write(root.join("eboot.bin"), [0x54, 0x14, 0xf5, 0xee, 0, 0, 0, 0]).unwrap();
        std::fs::write(
            root.join("sce_sys/param.json"),
            r#"{"contentId":"UP9000-PPSA99099_00-PROSPERO00000000"}"#,
        )
        .unwrap();
        let result = scan(&root.to_string_lossy());
        assert!(result.valid);
        assert!(!result.fpkg_ready);
        assert!(result.fpkg_blockers[0].contains("decrypted"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_configured_decrypted_subfolder_for_native_fpkg() {
        let root = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-fpkg-custom-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::create_dir_all(root.join("prepared/modules")).unwrap();
        std::fs::write(root.join("eboot.bin"), [0x54, 0x14, 0xf5, 0xee, 0, 0, 0, 0]).unwrap();
        std::fs::write(
            root.join("prepared/modules/eboot.bin"),
            [0x7f, b'E', b'L', b'F', 0, 0, 0, 0],
        )
        .unwrap();
        std::fs::write(
            root.join("sce_sys/param.json"),
            r#"{"contentId":"UP9000-PPSA99099_00-PROSPERO00000000"}"#,
        )
        .unwrap();

        let default_scan = scan(&root.to_string_lossy());
        assert!(!default_scan.fpkg_ready);
        let configured_scan =
            scan_with_decrypted_subfolder(&root.to_string_lossy(), "prepared/modules");
        assert!(configured_scan.valid);
        assert!(
            configured_scan.fpkg_ready,
            "{:?}",
            configured_scan.fpkg_blockers
        );
        assert_eq!(configured_scan.fpkg_module_count, 1);
        let lightweight = fpkg_readiness(&root.to_string_lossy(), "prepared/modules").unwrap();
        assert!(lightweight.ready, "{:?}", lightweight.blockers);
        assert_eq!(lightweight.module_count, configured_scan.fpkg_module_count);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_fpkg_subfolder_does_not_invalidate_the_whole_dump() {
        let root = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-invalid-fpkg-option-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::write(root.join("eboot.bin"), [0x7f, b'E', b'L', b'F']).unwrap();
        std::fs::write(
            root.join("sce_sys/param.json"),
            r#"{"contentId":"UP9000-PPSA99099_00-PROSPERO00000000"}"#,
        )
        .unwrap();

        let result = scan_with_decrypted_subfolder(&root.to_string_lossy(), "../unsafe");
        assert!(result.valid);
        assert!(!result.fpkg_ready);
        assert!(result.fpkg_blockers[0].contains("ruta relativa segura"));
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
        assert_eq!(tool_for_input(MODE_VERIFY, "game.ffpkg"), Some("ufs2tool"));
        assert_eq!(tool_for_input(MODE_VERIFY, "game.exfat"), Some("mkpfs"));
    }

    #[test]
    fn maps_recent_ps5_modes_to_bundled_engines() {
        assert!(is_mode(MODE_NATIVE_FPKG));
        assert!(is_mode(MODE_EXFAT_FPKG));
        assert!(is_mode(MODE_LZ4));
        assert_eq!(tool_for(MODE_NATIVE_FPKG), Some("prospero"));
        assert_eq!(tool_for(MODE_EXFAT_FPKG), Some("prospero"));
        assert_eq!(tool_for(MODE_LZ4), Some("ampr"));
        assert_eq!(output_ext(MODE_NATIVE_FPKG), Some("pkg"));
        assert_eq!(output_ext(MODE_EXFAT_FPKG), Some("pkg"));
        assert_eq!(output_ext(MODE_LZ4), None);
        assert!(writes_directory(MODE_LZ4));
    }

    #[test]
    fn builds_safe_fpkg_engine_options() {
        let options = BTreeMap::from([
            ("decrypted_subfolder".into(), "decrypted/modules".into()),
            ("embedded_right".into(), "true".into()),
        ]);
        let args = fpkg_convert_args("game", "game.pkg", &options).unwrap();
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--decrypted-subfolder", "decrypted/modules"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--embedded-right", "true"]));
    }

    #[test]
    fn rejects_unsafe_fpkg_subfolder() {
        let options = BTreeMap::from([("decrypted_subfolder".into(), "../outside".into())]);
        assert!(fpkg_convert_args("game", "game.pkg", &options).is_err());
    }

    #[test]
    fn builds_and_validates_lz4_profile_arguments() {
        let options = BTreeMap::from([("profile".into(), "maximum".into())]);
        let args = lz4_convert_args("game", "game-lz4", &options).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["--profile", "maximum"]));

        let invalid = BTreeMap::from([("profile".into(), "maximum; remove".into())]);
        assert!(lz4_convert_args("game", "game-lz4", &invalid).is_err());
    }

    #[test]
    fn blocks_ps5_output_inside_the_source_tree() {
        let base = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-output-guard-{}",
            std::process::id()
        ));
        let source = base.join("game");
        let sibling = base.join("converted");
        let nested = source.join("new/output/game.ffpkg");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&sibling).unwrap();

        assert!(validate_output_location(&source, &sibling).is_ok());
        assert!(validate_output_location(&source, &nested).is_err());
        assert!(validate_output_location(&source, &source).is_err());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn probes_space_on_the_configured_output_disk() {
        let base =
            std::env::temp_dir().join(format!("romforge-studio-ps5-space-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        let result = output_space(&base.join("future/output")).unwrap();
        assert!(result.available_bytes > 0);
        assert_eq!(PathBuf::from(result.location), base);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn reads_logical_extraction_size_without_decompressing_pfs() {
        let image = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-logical-{}.ffpfsc",
            std::process::id()
        ));
        let logical_bytes = 20 * GIB;
        let mut data = vec![0u8; 8192];
        data[0x00..0x08].copy_from_slice(&2i64.to_le_bytes());
        data[0x08..0x10].copy_from_slice(&PFS_MAGIC.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&4096u32.to_le_bytes());
        data[0x30..0x38].copy_from_slice(&2i64.to_le_bytes());
        data[0x40..0x48].copy_from_slice(&1i64.to_le_bytes());
        let inode = 4096 + 0xa8;
        data[inode..inode + 2].copy_from_slice(&INODE_MODE_FILE.to_le_bytes());
        data[inode + 4..inode + 8].copy_from_slice(&INODE_FLAG_COMPRESSED.to_le_bytes());
        data[inode + 8..inode + 16].copy_from_slice(&128i64.to_le_bytes());
        data[inode + 16..inode + 24].copy_from_slice(&(logical_bytes as i64).to_le_bytes());
        std::fs::write(&image, data).unwrap();

        assert_eq!(pfs_logical_file_bytes(&image).unwrap(), logical_bytes);
        let requirements = image_space_requirements(&image).unwrap();
        assert_eq!(requirements[MODE_EXTRACT], 22 * GIB);
        assert!(!requirements.contains_key(MODE_COMPRESS));
        let _ = std::fs::remove_file(image);
    }

    #[test]
    fn estimates_each_supported_exfat_image_operation() {
        let image = std::env::temp_dir().join(format!(
            "romforge-studio-ps5-image-space-{}.exfat",
            std::process::id()
        ));
        std::fs::write(&image, vec![0u8; 1024]).unwrap();

        let requirements = image_space_requirements(&image).unwrap();
        assert_eq!(requirements[MODE_COMPRESS], GIB + 1024);
        assert_eq!(requirements[MODE_EXTRACT], GIB + 1024);
        assert_eq!(requirements[MODE_EXFAT_FPKG], 2 * GIB + 2048);
        let _ = std::fs::remove_file(image);
    }

    #[cfg(windows)]
    #[test]
    fn output_guard_compares_windows_paths_case_insensitively() {
        assert!(path_starts_with(
            Path::new(r"C:\Games\PPSA00001\converted"),
            Path::new(r"c:\games\ppsa00001")
        ));
        assert!(!path_starts_with(
            Path::new(r"C:\Games\PPSA000010\converted"),
            Path::new(r"c:\games\ppsa00001")
        ));
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
