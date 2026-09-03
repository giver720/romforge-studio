use base64::Engine;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

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
            for collection in ["Named_Boxarts", "Named_Titles", "Named_Snaps"] {
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
                if let Some(bytes) = match client.as_ref() {
                    Some(client) => download(client, &url).await,
                    _ => None,
                } {
                    if let Some(parent) = cache.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&cache, &bytes);
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
}
