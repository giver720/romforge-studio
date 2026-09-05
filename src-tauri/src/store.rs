use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use std::sync::Mutex;

static ACTIVE_DOWNLOAD: Mutex<Option<tokio::sync::watch::Sender<bool>>> = Mutex::new(None);

struct DownloadGuard;
struct PartialFile(PathBuf);
struct PartialPackage(PathBuf);
impl Drop for PartialPackage {
    fn drop(&mut self) {
        // This directory is created exclusively for this operation below.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
impl Drop for PartialFile {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}
impl Drop for DownloadGuard {
    fn drop(&mut self) { if let Ok(mut active) = ACTIVE_DOWNLOAD.lock() { *active = None; } }
}

async fn cancellable<T>(work: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    let (sender, mut cancel) = tokio::sync::watch::channel(false);
    {
        let mut active = ACTIVE_DOWNLOAD.lock().map_err(|_| anyhow!("No se pudo iniciar la descarga"))?;
        if active.is_some() { return Err(anyhow!("Ya hay una descarga de la Store en marcha")); }
        *active = Some(sender);
    }
    let _guard = DownloadGuard;
    tokio::select! {
        biased;
        _ = cancel.changed() => Err(anyhow!("Descarga cancelada")),
        result = work => result,
    }
}

#[tauri::command]
pub fn cancel_store_download() -> Result<(), String> {
    let active = ACTIVE_DOWNLOAD.lock().map_err(|_| "No se pudo cancelar")?;
    if let Some(sender) = active.as_ref() { let _ = sender.send(true); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Uses the public catalog endpoint; run explicitly with network access"]
    async fn live_catalog_endpoint() {
        let catalog = fetch_store_catalog().await.unwrap();
        assert_eq!(catalog["schema_version"], 1);
        let entries = catalog["entries"].as_array().unwrap();
        assert!(entries.len() > 2000);
        for platform in ["3ds", "wii", "wiiu", "switch", "psvita", "psp", "ps4", "ps5"] {
            assert!(entries.iter().any(|e| e["platforms"].as_array().unwrap().iter().any(|p| p == platform)));
        }
    }

    #[tokio::test]
    #[ignore = "Downloads a small public metadata file without executing it"]
    async fn live_download_and_checksum_failure() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("romforge-store-test-{stamp}"));
        std::fs::create_dir(&root).unwrap();
        let _cleanup = PartialPackage(root.clone());
        let url = "https://raw.githubusercontent.com/giver720/romforge-studio/403c546b33459b603d15ca6cde0e5dafe651b3ed/tools/homebrew-sources.json";
        let output = download_homebrew_inner(|_| {}, url.into(), "sources.json".into(), root.to_string_lossy().into(), None).await.unwrap();
        let bytes = std::fs::read(&output).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["sources"].as_array().unwrap().len(), 10);
        let digest = format!("{:x}", Sha256::digest(&bytes));
        download_homebrew_inner(|_| {}, url.into(), "verified.json".into(), root.to_string_lossy().into(), Some(digest)).await.unwrap();
        let error = download_homebrew_inner(|_| {}, url.into(), "bad.json".into(), root.to_string_lossy().into(), Some("0".repeat(64))).await.unwrap_err();
        assert!(error.contains("SHA-256"));
        assert!(!root.join("bad.json").exists());
        assert!(!root.join("bad.json.part").exists());
        assert!(download_homebrew_inner(|_| {}, url.into(), "sources.json".into(), root.to_string_lossy().into(), None).await.is_err());
        assert_eq!(std::fs::read(output).unwrap(), bytes);
    }

    #[test]
    fn manifest_paths_stay_relative() {
        for path in ["/etc/passwd", "C:/test", "../test", "apps/../../test", "apps//test", "apps/./test", "apps/file:stream", "apps/test\0"] {
            assert!(validate_manifest_path(path).is_err(), "{path}");
        }
        assert!(validate_manifest_path("wiiu/apps/100 Boxes/main.rpx").is_ok());
    }

    #[tokio::test]
    #[ignore = "Downloads a small HBAS package without executing it"]
    async fn live_hbas_package() {
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("romforge-hbas-test-{stamp}"));
        std::fs::create_dir(&root).unwrap();
        let _cleanup = PartialPackage(root.clone());
        let url = "https://wiiu.cdn.fortheusers.org/packages/100_Boxes_Wiiu/manifest.install";
        let files = Mutex::new(std::collections::HashSet::new());
        let output = download_hbas_inner(|event| { files.lock().unwrap().insert(event.filename); }, url.into(), root.to_string_lossy().into(), "100 Boxes".into()).await.unwrap();
        let package = PathBuf::from(output).join("wiiu/apps/100_Boxes_Wiiu");
        for name in ["meta.xml", "icon.png", "100_Boxes.rpx"] {
            assert!(std::fs::metadata(package.join(name)).unwrap().len() > 0);
        }
        assert_eq!(files.lock().unwrap().len(), 3);
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        assert!(download_hbas_inner(|_| {}, url.into(), root.to_string_lossy().into(), "100 Boxes".into()).await.is_err());
    }

    #[tokio::test]
    async fn cancellation_stops_work_and_releases_slot() {
        let (started, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(cancellable(async move {
            let _ = started.send(());
            std::future::pending::<Result<()>>().await
        }));
        ready.await.unwrap();
        assert!(cancellable(async { Ok(()) }).await.is_err());
        cancel_store_download().unwrap();
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), task).await.unwrap().unwrap();
        assert_eq!(result.unwrap_err().to_string(), "Descarga cancelada");
        assert!(cancellable(async { Ok(()) }).await.is_ok());
    }
}

#[tauri::command]
pub async fn fetch_store_catalog() -> Result<serde_json::Value, String> {
    let result = async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .https_only(true)
            .build()?;
        let response = client.get("https://raw.githubusercontent.com/giver720/romforge-studio/main/public/store/catalog.json")
            .send().await?.error_for_status()?;
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if bytes.len() + chunk.len() > 20 * 1024 * 1024 {
                return Err(anyhow!("El catálogo supera el tamaño permitido"));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok::<serde_json::Value, anyhow::Error>(serde_json::from_slice(&bytes)?)
    }.await;
    result.map_err(|e| e.to_string())
}

#[derive(serde::Serialize, Clone)]
struct DownloadProgress {
    filename: String,
    received: u64,
    total: Option<u64>,
}

fn safe_filename(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." || trimmed.contains(['/', '\\', ':', '\0']) {
        return Err(anyhow!("Nombre de archivo no válido"));
    }
    Ok(trimmed.to_string())
}

fn validate_manifest_path(relative: &str) -> Result<()> {
    if relative.is_empty() || relative.starts_with('/') || relative.contains([':', '\0', '\\']) || relative.split('/').any(|part| part.is_empty() || part == ".." || part == ".") {
        return Err(anyhow!("Ruta insegura en manifiesto"));
    }
    Ok(())
}

async fn fetch_file(emit: &impl Fn(DownloadProgress), client: &reqwest::Client, url: &str, path: &Path, label: &str) -> Result<()> {
    let response = client.get(url).send().await?.error_for_status()?;
    let total = response.content_length();
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(path).await?;
    let mut received = 0u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        received += chunk.len() as u64;
        emit(DownloadProgress { filename: label.to_string(), received, total });
    }
    tokio::io::AsyncWriteExt::flush(&mut file).await?;
    Ok(())
}

#[tauri::command]
pub async fn download_homebrew(
    app: AppHandle,
    url: String,
    filename: String,
    destination_dir: String,
    sha256: Option<String>,
) -> Result<String, String> {
    download_homebrew_inner(move |progress| { let _ = app.emit("store://download", progress); }, url, filename, destination_dir, sha256).await
}

async fn download_homebrew_inner(
    emit: impl Fn(DownloadProgress), url: String, filename: String,
    destination_dir: String, sha256: Option<String>,
) -> Result<String, String> {
    let result = cancellable(async move {
        if !url.starts_with("https://") {
            return Err(anyhow!("Solo se permiten descargas HTTPS"));
        }
        let filename = safe_filename(&filename)?;
        let dir = PathBuf::from(destination_dir);
        std::fs::create_dir_all(&dir).context("No se pudo crear la carpeta de descarga")?;
        let final_path = dir.join(&filename);
        let temp_path = final_path.with_extension(format!("{}.part", final_path.extension().and_then(|x| x.to_str()).unwrap_or("download")));
        if final_path.exists() { return Err(anyhow!("El archivo ya existe. Elige otra carpeta para conservar ambas copias.")); }
        if temp_path.exists() { return Err(anyhow!("Existe una descarga parcial previa. Elige otra carpeta.")); }
        let _partial = PartialFile(temp_path.clone());
        let client = reqwest::Client::builder().https_only(true).connect_timeout(std::time::Duration::from_secs(20)).build()?;
        let response = client.get(&url).send().await?.error_for_status()?;
        let total = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::OpenOptions::new().write(true).create_new(true).open(&temp_path).await?;
        let mut hash = Sha256::new();
        let mut received = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            hash.update(&chunk);
            received += chunk.len() as u64;
            emit(DownloadProgress { filename: filename.clone(), received, total });
        }
        tokio::io::AsyncWriteExt::flush(&mut file).await?;
        drop(file);
        if let Some(expected) = sha256.filter(|v| !v.trim().is_empty()) {
            let actual = format!("{:x}", hash.finalize());
            if actual != expected.trim().to_lowercase() {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err(anyhow!("Checksum SHA-256 no coincide"));
            }
        }
        tokio::fs::rename(&temp_path, &final_path).await?;
        Ok(final_path.to_string_lossy().to_string())
    }).await;
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn download_hbas_package(
    app: AppHandle,
    manifest_url: String,
    destination_dir: String,
    package_name: String,
) -> Result<String, String> {
    download_hbas_inner(move |progress| { let _ = app.emit("store://download", progress); }, manifest_url, destination_dir, package_name).await
}

async fn download_hbas_inner(
    emit: impl Fn(DownloadProgress), manifest_url: String, destination_dir: String, package_name: String,
) -> Result<String, String> {
    let result = cancellable(async move {
        if !manifest_url.starts_with("https://") { return Err(anyhow!("Solo se permiten manifiestos HTTPS")); }
        let package_name = safe_filename(&package_name)?;
        let base = manifest_url.rsplit_once('/').map(|(p, _)| p).ok_or_else(|| anyhow!("Manifiesto inválido"))?;
        let client = reqwest::Client::builder().https_only(true).connect_timeout(std::time::Duration::from_secs(20)).build()?;
        let manifest = client.get(&manifest_url).send().await?.error_for_status()?.text().await?;
        let destination = PathBuf::from(destination_dir);
        tokio::fs::create_dir_all(&destination).await?;
        let final_root = destination.join(&package_name);
        if final_root.exists() { return Err(anyhow!("El paquete ya existe. Elige otra carpeta para conservar ambas copias.")); }
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
        let root = destination.join(format!(".romforge-{stamp}.part"));
        tokio::fs::create_dir(&root).await?;
        let _partial = PartialPackage(root.clone());
        let mut count = 0;
        for line in manifest.lines() {
            let Some(relative) = line.strip_prefix("U: ") else { continue };
            let relative = relative.trim().replace('\\', "/");
            validate_manifest_path(&relative)?;
            let target = root.join(&relative);
            if let Some(parent) = target.parent() { tokio::fs::create_dir_all(parent).await?; }
            let url = format!("{}/{}", base, relative);
            fetch_file(&emit, &client, &url, &target, &relative).await?;
            count += 1;
        }
        if count == 0 { return Err(anyhow!("El manifiesto no contiene archivos descargables")); }
        if final_root.exists() { return Err(anyhow!("La carpeta de destino ya existe")); }
        tokio::fs::rename(&root, &final_root).await?;
        Ok(final_root.to_string_lossy().to_string())
    }).await;
    result.map_err(|e| e.to_string())
}
