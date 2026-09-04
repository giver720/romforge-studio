use base64::Engine;
use serde::Serialize;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;

const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_INDEX_BYTES: usize = 4 * 1024 * 1024;
const INDEX_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const COLLECTIONS: [&str; 3] = ["Named_Boxarts", "Named_Titles", "Named_Snaps"];
static CATALOG_INDEXES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[derive(Clone, Debug, Serialize)]
pub struct GameArtwork {
    pub data_url: Option<String>,
    /// local | cache | libretro
    pub source: Option<String>,
    pub title: Option<String>,
}

fn child_case_insensitive(parent: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(parent)
        .ok()?
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
        })
        .map(|entry| entry.path())
}

fn nested_case_insensitive(root: &Path, parts: &[&str]) -> Option<PathBuf> {
    let mut current = root.to_path_buf();
    for part in parts {
        current = child_case_insensitive(&current, part)?;
    }
    Some(current)
}

fn first_image(dir: &Path, stems: &[&str]) -> Option<PathBuf> {
    for stem in stems {
        for extension in ["png", "jpg", "jpeg", "webp"] {
            let wanted = format!("{stem}.{extension}");
            if let Some(path) = child_case_insensitive(dir, &wanted).filter(|path| path.is_file()) {
                return Some(path);
            }
        }
    }
    None
}

fn local_artwork(input: &Path) -> Option<PathBuf> {
    if input.is_dir() {
        for parts in [
            &["sce_sys", "icon0.png"][..],
            &["PS3_GAME", "ICON0.PNG"][..],
            &["PSP_GAME", "ICON0.PNG"][..],
        ] {
            if let Some(path) = nested_case_insensitive(input, parts).filter(|path| path.is_file())
            {
                return Some(path);
            }
        }
        return first_image(input, &["cover", "folder", "icon0", "front"]);
    }

    let parent = input.parent()?;
    let stem = input.file_stem()?.to_string_lossy();
    for extension in ["png", "jpg", "jpeg", "webp"] {
        for suffix in ["", "-cover", "_cover"] {
            let wanted = format!("{stem}{suffix}.{extension}");
            if let Some(path) =
                child_case_insensitive(parent, &wanted).filter(|path| path.is_file())
            {
                return Some(path);
            }
        }
    }
    None
}

fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn as_data_url(bytes: &[u8]) -> Option<String> {
    let mime = image_mime(bytes)?;
    Some(format!(
        "data:{mime};base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn read_image(path: &Path) -> Option<Vec<u8>> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() == 0 || meta.len() > MAX_IMAGE_BYTES as u64 {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    image_mime(&bytes)?;
    Some(bytes)
}

fn playlists(system: &str) -> &'static [&'static str] {
    match system {
        "psx" => &["Sony - PlayStation"],
        "ps2" | "ps2cd" => &["Sony - PlayStation 2"],
        "psp" => &["Sony - PlayStation Portable"],
        "ps3" => &[
            "Sony - PlayStation 3",
            "Sony - PlayStation 3 (Downloadable)",
        ],
        "saturn" => &["Sega - Saturn"],
        "dreamcast" => &["Sega - Dreamcast"],
        "segacd" => &["Sega - Mega-CD - Sega CD"],
        "pcecd" => &["NEC - PC Engine CD - TurboGrafx-CD"],
        "neogeocd" => &["SNK - Neo Geo CD"],
        "3do" => &["The 3DO Company - 3DO"],
        "cdi" => &["Philips - CD-i"],
        "pcfx" => &["NEC - PC-FX"],
        "cd32" => &["Commodore - Amiga CD32"],
        "3ds" => &["Nintendo - Nintendo 3DS"],
        "xbox" => &["Microsoft - Xbox"],
        "xbox360" => &["Microsoft - Xbox 360"],
        "wii" => &["Nintendo - Wii", "Nintendo - GameCube"],
        _ => &[],
    }
}

fn clean_title(input: &Path) -> String {
    let raw = if input.is_dir() {
        input.file_name()
    } else {
        input.file_stem()
    }
    .unwrap_or_default()
    .to_string_lossy();
    raw.trim_end_matches(".compact")
        .trim_end_matches(" (Disc 1)")
        .trim()
        .to_string()
}

fn thumbnail_name(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if ['&', '*', '/', ':', '`', '<', '>', '?', '\\', '|', '"'].contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect()
}

fn candidate_names(title: &str) -> Vec<String> {
    let exact = thumbnail_name(title);
    let short = title.split(" (").next().unwrap_or(title).trim();
    let short = thumbnail_name(short);
    let mut names = vec![exact.clone()];
    if !short.is_empty() && short != exact {
        names.push(short.clone());
    }
    if !short.is_empty() {
        for region in ["USA", "Europe", "Japan"] {
            let regional = format!("{short} ({region})");
            if !names.contains(&regional) {
                names.push(regional);
            }
        }
    }
    names
}

fn percent_encode(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (*byte as char).to_string()
            }
            byte => format!("%{byte:02X}"),
        })
        .collect()
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            decoded.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn comparison_tokens(title: &str) -> Vec<String> {
    let title = title.split(" (").next().unwrap_or(title);
    let normalized: String = title
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect();
    normalized.split_whitespace().map(str::to_string).collect()
}

fn fuzzy_score(query: &[String], candidate: &[String]) -> f32 {
    if query.len() < 2 || candidate.is_empty() {
        return 0.0;
    }
    let matches = query
        .iter()
        .filter(|token| candidate.contains(token))
        .count();
    let coverage = matches as f32 / query.len() as f32;
    let precision = matches as f32 / candidate.len() as f32;
    coverage * 0.75 + precision * 0.25
}

fn fuzzy_name_from_index(html: &str, title: &str) -> Option<String> {
    let query = comparison_tokens(title);
    let mut best: Option<(f32, String)> = None;
    for fragment in html.split("href=\"").skip(1) {
        let Some(href) = fragment.split('"').next() else {
            continue;
        };
        if !href.to_ascii_lowercase().ends_with(".png") {
            continue;
        }
        let Some(decoded) = percent_decode(href) else {
            continue;
        };
        let name = decoded[..decoded.len() - 4].to_string();
        let mut score = fuzzy_score(&query, &comparison_tokens(&name));
        if name.contains("(Europe") {
            score += 0.015;
        } else if name.contains("(USA") {
            score += 0.01;
        }
        if score >= 0.82 && best.as_ref().map_or(true, |current| score > current.0) {
            best = Some((score, name));
        }
    }
    best.map(|(_, name)| name)
}

fn cache_path(system: &str, url: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    system.hash(&mut hasher);
    url.hash(&mut hasher);
    crate::settings::config_dir()
        .join("covers")
        .join(format!("{:016x}.image", hasher.finish()))
}

async fn download(client: &reqwest::Client, url: &str) -> Option<Vec<u8>> {
    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    if response.content_length().unwrap_or(0) > MAX_IMAGE_BYTES as u64 {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES || image_mime(&bytes).is_none() {
        return None;
    }
    Some(bytes.to_vec())
}

async fn download_index(client: &reqwest::Client, url: &str) -> Option<String> {
    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success()
        || response.content_length().unwrap_or(0) > MAX_INDEX_BYTES as u64
    {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_INDEX_BYTES {
        return None;
    }
    String::from_utf8(bytes.to_vec()).ok()
}

fn fresh_index(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    if metadata.len() == 0 || metadata.len() > MAX_INDEX_BYTES as u64 {
        return None;
    }
    let modified = metadata.modified().ok()?;
    if SystemTime::now().duration_since(modified).ok()? > INDEX_MAX_AGE {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

async fn catalog_index(client: &reqwest::Client, url: &str) -> Option<String> {
    let indexes = CATALOG_INDEXES.get_or_init(|| Mutex::new(HashMap::new()));
    let mut indexes = indexes.lock().await;
    if let Some(index) = indexes.get(url) {
        return Some(index.clone());
    }

    let cache = cache_path("catalog", url).with_extension("html");
    if let Some(index) = fresh_index(&cache) {
        indexes.insert(url.to_string(), index.clone());
        return Some(index);
    }

    let index = download_index(client, url).await?;
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(cache, index.as_bytes());
    indexes.insert(url.to_string(), index.clone());
    Some(index)
}

fn metadata_title(input: &Path, system: &str) -> Option<String> {
    if !input.is_dir() {
        return None;
    }
    match system {
        "ps5" => crate::ps5::scan(&input.to_string_lossy()).title,
        "ps3" => crate::ps3::scan(&input.to_string_lossy()).title,
        _ => None,
    }
}

pub async fn resolve(input: &str, system: &str, online: bool) -> GameArtwork {
    let input = Path::new(input);
    let title = metadata_title(input, system);
    if let Some(bytes) = local_artwork(input).and_then(|path| read_image(&path)) {
        return GameArtwork {
            data_url: as_data_url(&bytes),
            source: Some("local".into()),
            title,
        };
    }

    let lookup_title = clean_title(input);
    if lookup_title.is_empty() || playlists(system).is_empty() {
        return GameArtwork {
            data_url: None,
            source: None,
            title,
        };
    }
    let query_cache = cache_path(system, &format!("query:{lookup_title}"));
    if let Some(bytes) = read_image(&query_cache) {
        return GameArtwork {
            data_url: as_data_url(&bytes),
            source: Some("cache".into()),
            title,
        };
    }
    let client = if online {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(7))
            .user_agent("CHD-Studio/1.8 artwork resolver")
            .build()
            .ok()
    } else {
        None
    };
    for playlist in playlists(system) {
        for name in candidate_names(&lookup_title) {
            for collection in COLLECTIONS {
                let url = format!(
                    "https://thumbnails.libretro.com/{}/{}/{}.png",
                    percent_encode(playlist),
                    collection,
                    percent_encode(&name)
                );
                let cache = cache_path(system, &url);
                if let Some(bytes) = read_image(&cache) {
                    return GameArtwork {
                        data_url: as_data_url(&bytes),
                        source: Some("cache".into()),
                        title,
                    };
                }
            }
        }
    }
    if let Some(client) = client.as_ref() {
        for playlist in playlists(system) {
            for collection in COLLECTIONS {
                let index_url = format!(
                    "https://thumbnails.libretro.com/{}/{}/",
                    percent_encode(playlist),
                    collection
                );
                let Some(index) = catalog_index(client, &index_url).await else {
                    continue;
                };
                let Some(name) = fuzzy_name_from_index(&index, &lookup_title) else {
                    continue;
                };
                let url = format!("{index_url}{}.png", percent_encode(&name));
                let cache = cache_path(system, &url);
                if let Some(bytes) = read_image(&cache) {
                    let _ = std::fs::write(&query_cache, &bytes);
                    return GameArtwork {
                        data_url: as_data_url(&bytes),
                        source: Some("cache".into()),
                        title,
                    };
                }
                let Some(bytes) = download(client, &url).await else {
                    continue;
                };
                if let Some(parent) = cache.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&cache, &bytes);
                let _ = std::fs::write(&query_cache, &bytes);
                return GameArtwork {
                    data_url: as_data_url(&bytes),
                    source: Some("libretro".into()),
                    title,
                };
            }
        }

        // Si el servidor deja de publicar el índice, conserva la búsqueda
        // directa como respaldo. Solo se ejecuta después del camino rápido.
        for playlist in playlists(system) {
            for name in candidate_names(&lookup_title) {
                for collection in COLLECTIONS {
                    let url = format!(
                        "https://thumbnails.libretro.com/{}/{}/{}.png",
                        percent_encode(playlist),
                        collection,
                        percent_encode(&name)
                    );
                    let cache = cache_path(system, &url);
                    let Some(bytes) = download(client, &url).await else {
                        continue;
                    };
                    if let Some(parent) = cache.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&cache, &bytes);
                    let _ = std::fs::write(&query_cache, &bytes);
                    return GameArtwork {
                        data_url: as_data_url(&bytes),
                        source: Some("libretro".into()),
                        title,
                    };
                }
            }
        }
    }
    GameArtwork {
        data_url: None,
        source: None,
        title,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_ps5_icon_before_generic_cover() {
        let root = std::env::temp_dir().join(format!("chd-studio-art-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("sce_sys")).unwrap();
        std::fs::write(root.join("sce_sys/icon0.png"), b"icon").unwrap();
        std::fs::write(root.join("cover.jpg"), b"cover").unwrap();
        assert_eq!(
            local_artwork(&root).unwrap(),
            root.join("sce_sys/icon0.png")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn applies_libretro_filename_rules_and_short_fallback() {
        assert_eq!(thumbnail_name("Q*Bert: Test"), "Q_Bert_ Test");
        assert_eq!(
            candidate_names("Game (USA)"),
            vec![
                "Game (USA)".to_string(),
                "Game".to_string(),
                "Game (Europe)".to_string(),
                "Game (Japan)".to_string(),
            ]
        );
    }

    #[test]
    fn finds_catalog_title_when_filename_omits_a_word_and_region() {
        let html = r#"<a href="Dragon%20Ball%20Z%20-%20Shin%20Budokai%20(USA).png">other</a>
            <a href="Dragon%20Ball%20Z%20-%20Tenkaichi%20Tag%20Team%20(Europe)%20(En%2CFr%2CDe%2CEs%2CIt).png">wanted</a>"#;
        assert_eq!(
            fuzzy_name_from_index(html, "Dragon Ball Z - Tag Team"),
            Some("Dragon Ball Z - Tenkaichi Tag Team (Europe) (En,Fr,De,Es,It)".to_string())
        );
    }

    #[test]
    fn refuses_a_weak_catalog_match() {
        let html = r#"<a href="Dragon%20Ball%20Z%20-%20Shin%20Budokai%20(USA).png">other</a>"#;
        assert_eq!(
            fuzzy_name_from_index(html, "Dragon Ball Z - Tag Team"),
            None
        );
    }

    #[tokio::test]
    #[ignore = "usa la red publica de miniaturas de Libretro"]
    async fn resolves_shortened_psp_title_online() {
        let artwork = resolve("Dragon Ball Z - Tag Team.iso", "psp", true).await;
        assert!(artwork.data_url.is_some());
        assert!(matches!(
            artwork.source.as_deref(),
            Some("libretro" | "cache")
        ));

        let first = std::time::Instant::now();
        assert!(resolve("Dragon Ball Z Tag Team.iso", "psp", true)
            .await
            .data_url
            .is_some());
        let first_elapsed = first.elapsed();
        let second = std::time::Instant::now();
        assert!(resolve("Dragon Ball Z Tag-Team.iso", "psp", true)
            .await
            .data_url
            .is_some());
        eprintln!(
            "primer catálogo: {:?}; catálogo reutilizado: {:?}",
            first_elapsed,
            second.elapsed()
        );
    }
}
