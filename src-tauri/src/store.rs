use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

#[derive(serde::Serialize, Clone)]
struct DownloadProgress {
    filename: String,
    received: u64,
    total: Option<u64>,
}

fn safe_filename(name: &str) -> Result<String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." || trimmed.contains(['/', '\\']) {
        return Err(anyhow!("Nombre de archivo no válido"));
    }
    Ok(trimmed.to_string())
}

#[tauri::command]
pub async fn download_homebrew(
    app: AppHandle,
    url: String,
    filename: String,
    destination_dir: String,
    sha256: Option<String>,
) -> Result<String, String> {
    let result = async move {
        if !url.starts_with("https://") {
            return Err(anyhow!("Solo se permiten descargas HTTPS"));
        }
        let filename = safe_filename(&filename)?;
        let dir = PathBuf::from(destination_dir);
        std::fs::create_dir_all(&dir).context("No se pudo crear la carpeta de descarga")?;
        let final_path = dir.join(&filename);
        let temp_path = final_path.with_extension(format!("{}.part", final_path.extension().and_then(|x| x.to_str()).unwrap_or("download")));
        let response = reqwest::Client::new().get(&url).send().await?.error_for_status()?;
        let total = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&temp_path).await?;
        let mut hash = Sha256::new();
        let mut received = 0u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            hash.update(&chunk);
            received += chunk.len() as u64;
            let _ = app.emit("store://download", DownloadProgress { filename: filename.clone(), received, total });
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
    }.await;
    result.map_err(|e| e.to_string())
}
