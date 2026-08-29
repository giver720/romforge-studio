use crate::chdman;
use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncReadExt, BufReader};

pub const EV_JOB: &str = "job://update";
pub const EV_TOAST: &str = "app://toast";

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub input: String,
    pub input_name: String,
    pub output: String,
    /// Segunda salida (el .bin que acompana a un .cue al extraer)
    pub output_extra: Option<String>,
    /// Herramienta que ejecuta el trabajo: chdman | nsz | 4nxci
    pub tool: String,
    /// Accion concreta dentro de esa herramienta
    pub mode: String,
    /// Id del perfil de sistema, solo para mostrarlo en la interfaz
    pub system: String,
    pub codecs: Vec<String>,
    pub hunk_size: Option<u32>,
    pub unit_size: Option<u32>,
    /// queued | running | done | error | canceled
    pub status: String,
    pub progress: f32,
    pub phase: String,
    pub ratio: Option<f32>,
    pub message: Option<String>,
    /// pending | running | passed | basic | failed | not_applicable
    pub verification: String,
    pub verification_message: Option<String>,
    pub log: Vec<String>,
    pub input_size: u64,
    pub output_size: u64,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
}

impl Job {
    pub fn new(input: String, output: String, tool: String, mode: String, system: String) -> Self {
        let input_name = Path::new(&input)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| input.clone());
        let input_size = std::fs::metadata(&input).map(|m| m.len()).unwrap_or(0);
        Self {
            id: format!("{}-{}", now_ms(), fastrand_hex()),
            input,
            input_name,
            output,
            output_extra: None,
            tool,
            mode,
            system,
            codecs: vec![],
            hunk_size: None,
            unit_size: None,
            status: "queued".into(),
            progress: 0.0,
            phase: "En cola".into(),
            ratio: None,
            message: None,
            verification: "pending".into(),
            verification_message: None,
            log: vec![],
            input_size,
            output_size: 0,
            started_at: None,
            finished_at: None,
        }
    }
}

/// Identificador corto y unico sin dependencias extra.
fn fastrand_hex() -> String {
    use std::sync::atomic::AtomicU64;
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    format!("{:x}", n.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ t)
}

#[derive(Default)]
pub struct AppState {
    pub settings: Mutex<Settings>,
    pub jobs: Mutex<Vec<Job>>,
    pub cancels: Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
    pub children: Mutex<std::collections::HashMap<String, u32>>,
    pub pumping: AtomicBool,
}

impl AppState {
    pub fn snapshot(&self) -> Vec<Job> {
        self.jobs.lock().unwrap().clone()
    }

    pub fn update<F: FnOnce(&mut Job)>(&self, id: &str, f: F) -> Option<Job> {
        let mut jobs = self.jobs.lock().unwrap();
        let job = jobs.iter_mut().find(|j| j.id == id)?;
        f(job);
        Some(job.clone())
    }
}

fn emit_job(app: &AppHandle, job: &Job) {
    let _ = app.emit(EV_JOB, job);
}

#[allow(dead_code)]
pub fn toast(app: &AppHandle, kind: &str, text: &str) {
    let _ = app.emit(EV_TOAST, serde_json::json!({ "kind": kind, "text": text }));
}

/// Reparte la construccion de argumentos segun la herramienta del trabajo.
fn build_args(job: &Job, s: &Settings) -> Vec<String> {
    match job.tool.as_str() {
        "nsz" => {
            let dir = out_dir_of(job);
            crate::switch::nsz_args(&job.mode, &job.input, &dir, s)
        }
        "4nxci" => crate::switch::nxci_args(&job.input, &out_dir_of(job), s),
        "z3ds" => crate::threeds::z3ds_args(&job.input, &job.output),
        "3dsconv" => crate::threeds::conv_args(&job.input, &out_dir_of(job), s),
        "iso2god" => crate::xbox360::args(&job.input, &job.output, s),
        "xiso" => match job.mode.as_str() {
            "folder2iso" => crate::xbox360::build_args(&job.input, &job.output),
            _ => crate::xbox360::extract_args(&job.input, &job.output, s),
        },
        "maxcso" => crate::psp::args(&job.mode, &job.input, &job.output, s),
        "wit" => crate::wii::wbfs_args(&job.input, &job.output, s),
        "dolphintool" => {
            // Carpeta propia para los temporales de Dolphin
            let user = crate::settings::config_dir().join("dolphin");
            let _ = std::fs::create_dir_all(&user);
            let user = user.to_string_lossy().to_string();
            if job.mode == "wiiverify" {
                crate::wii::verify_args(&job.input, &user)
            } else {
                crate::wii::convert_args(&job.mode, &job.input, &job.output, &user, s)
            }
        }
        "ps3iso" => match job.mode.as_str() {
            "ps3build" => crate::ps3::build_args(&job.input, &job.output, s.ps3_split_fat32),
            "ps3split" => crate::ps3::split_args(&job.input),
            // Al extraer no se parte nada: si luego se reconstruye el ISO, los
            // trozos sueltos confundirian a makeps3iso.
            _ => crate::ps3::extract_args(&job.input, &job.output, false),
        },
        _ => chdman_args(job, s),
    }
}

/// Los modos de comprobacion no generan archivo, asi que no hay nada que limpiar.
fn is_verify(mode: &str) -> bool {
    matches!(mode, "verify" | "wiiverify")
}

/// Algunos modos producen una carpeta en vez de un archivo suelto.
fn writes_directory(tool: &str) -> bool {
    tool == "iso2god"
}

fn mode_writes_directory(job: &Job) -> bool {
    writes_directory(&job.tool) || job.mode == "ps3extract" || job.mode == "iso2folder"
}

/// Suma recursiva del contenido de una carpeta, para poder informar del tamaño.
fn dir_size(path: &Path) -> u64 {
    let Ok(rd) = std::fs::read_dir(path) else {
        return 0;
    };
    rd.flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => dir_size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}

fn remove_path(path: &Path) {
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

#[derive(Debug)]
struct OutputEntry {
    target: PathBuf,
    backup: Option<PathBuf>,
}

/// Protege cualquier salida anterior hasta que la conversion y su verificacion
/// terminen. Todos los respaldos se crean al lado del original para que el
/// `rename` sea atomico y no cruce unidades.
#[derive(Debug, Default)]
struct OutputTransaction {
    entries: Vec<OutputEntry>,
}

impl OutputTransaction {
    fn protect(&mut self, target: PathBuf, overwrite: bool, job_id: &str) -> Result<(), String> {
        if !target.exists() {
            self.entries.push(OutputEntry {
                target,
                backup: None,
            });
            return Ok(());
        }
        if !overwrite {
            return Err(format!(
                "La salida ya existe: {}. Activa «Sobrescribir» o elige otra carpeta.",
                target.display()
            ));
        }
        let Some(parent) = target.parent() else {
            return Err("No se pudo determinar la carpeta de la salida existente".into());
        };
        let name = target
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "salida".into());
        let backup = parent.join(format!("{name}.chd-studio-backup-{job_id}"));
        if backup.exists() {
            return Err(format!(
                "Ya existe un respaldo pendiente: {}",
                backup.display()
            ));
        }
        std::fs::rename(&target, &backup).map_err(|error| {
            format!(
                "No se pudo proteger la salida anterior {}: {error}",
                target.display()
            )
        })?;
        self.entries.push(OutputEntry {
            target,
            backup: Some(backup),
        });
        Ok(())
    }

    fn begin(job: &Job, overwrite: bool) -> Result<Self, String> {
        if is_verify(&job.mode) || job.mode == "ps3split" {
            return Ok(Self::default());
        }

        let mut targets = vec![PathBuf::from(&job.output)];
        if let Some(extra) = &job.output_extra {
            targets.push(PathBuf::from(extra));
        }

        let mut tx = Self::default();
        for target in targets {
            if let Err(error) = tx.protect(target, overwrite, &job.id) {
                tx.rollback();
                return Err(error);
            }
        }
        Ok(tx)
    }

    fn rollback(&self) {
        for entry in self.entries.iter().rev() {
            if let Some(backup) = &entry.backup {
                remove_path(&entry.target);
                let _ = std::fs::rename(backup, &entry.target);
            } else {
                remove_path(&entry.target);
            }
        }
    }

    fn commit(&self) {
        for entry in &self.entries {
            if let Some(backup) = &entry.backup {
                remove_path(backup);
            }
        }
    }
}

#[derive(Debug)]
struct StagedOutput {
    execution_job: Job,
    root: Option<PathBuf>,
    final_parent: Option<PathBuf>,
}

impl StagedOutput {
    fn new(job: &Job, overwrite: bool) -> Result<Self, String> {
        if is_verify(&job.mode) || job.mode == "ps3split" {
            return Ok(Self {
                execution_job: job.clone(),
                root: None,
                final_parent: None,
            });
        }

        for target in std::iter::once(&job.output).chain(job.output_extra.iter()) {
            if Path::new(target).exists() && !overwrite {
                return Err(format!(
                    "La salida ya existe: {target}. Activa «Sobrescribir» o elige otra carpeta."
                ));
            }
        }

        let final_output = PathBuf::from(&job.output);
        let final_parent = final_output
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(&final_parent)
            .map_err(|e| format!("No se pudo crear la carpeta de destino: {e}"))?;
        let root = final_parent.join(format!(".chd-studio-stage-{}", job.id));
        if root.exists() {
            return Err(format!(
                "Ya existe una conversion temporal pendiente: {}",
                root.display()
            ));
        }
        std::fs::create_dir(&root)
            .map_err(|e| format!("No se pudo crear la carpeta temporal: {e}"))?;

        let mut execution_job = job.clone();
        let output_name = final_output
            .file_name()
            .ok_or_else(|| "La salida no tiene un nombre de archivo valido".to_string())?;
        execution_job.output = root.join(output_name).to_string_lossy().to_string();
        if let Some(extra) = &job.output_extra {
            let extra_name = Path::new(extra)
                .file_name()
                .ok_or_else(|| "La salida adicional no tiene un nombre valido".to_string())?;
            execution_job.output_extra = Some(root.join(extra_name).to_string_lossy().to_string());
        }

        Ok(Self {
            execution_job,
            root: Some(root),
            final_parent: Some(final_parent),
        })
    }

    fn cleanup(&self) {
        if let Some(root) = &self.root {
            let _ = std::fs::remove_dir_all(root);
        }
    }

    fn publish(&self, final_job: &Job, overwrite: bool) -> Result<Vec<PathBuf>, String> {
        let Some(root) = &self.root else {
            return Ok(vec![]);
        };
        let Some(final_parent) = &self.final_parent else {
            return Err("No se conoce la carpeta final del resultado".into());
        };
        let entries: Vec<PathBuf> = std::fs::read_dir(root)
            .map_err(|e| format!("No se pudo leer la salida temporal: {e}"))?
            .filter_map(|entry| entry.ok().map(|value| value.path()))
            .collect();
        if entries.is_empty() {
            return Err("La conversion temporal no produjo ningun resultado".into());
        }

        let mut transaction = OutputTransaction::begin(final_job, overwrite)?;
        let expected: Vec<PathBuf> = std::iter::once(PathBuf::from(&final_job.output))
            .chain(final_job.output_extra.iter().map(PathBuf::from))
            .collect();

        for source in &entries {
            let Some(name) = source.file_name() else {
                transaction.rollback();
                return Err("Una salida temporal no tiene nombre valido".into());
            };
            let target = final_parent.join(name);
            if !expected.iter().any(|path| path == &target) {
                if let Err(error) = transaction.protect(target, overwrite, &final_job.id) {
                    transaction.rollback();
                    return Err(error);
                }
            }
        }

        let targets: Vec<PathBuf> = entries
            .iter()
            .map(|source| final_parent.join(source.file_name().unwrap()))
            .collect();
        for (source, target) in entries.iter().zip(&targets) {
            if let Err(error) = std::fs::rename(source, target) {
                transaction.rollback();
                return Err(format!("No se pudo publicar {}: {error}", target.display()));
            }
        }

        transaction.commit();
        let _ = std::fs::remove_dir(root);
        Ok(targets)
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod output_transaction_tests {
    use super::*;

    fn fixture(name: &str) -> (PathBuf, Job) {
        let dir = std::env::temp_dir().join(format!(
            "chd-studio-output-transaction-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("input.iso");
        let output = dir.join("output.chd");
        std::fs::write(&input, b"source").unwrap();
        let job = Job::new(
            input.to_string_lossy().to_string(),
            output.to_string_lossy().to_string(),
            "chdman".into(),
            "createdvd".into(),
            "ps2".into(),
        );
        (dir, job)
    }

    #[test]
    fn rollback_restores_previous_output() {
        let (dir, job) = fixture("rollback");
        std::fs::write(&job.output, b"previous").unwrap();
        let tx = OutputTransaction::begin(&job, true).unwrap();
        std::fs::write(&job.output, b"partial").unwrap();
        tx.rollback();
        assert_eq!(std::fs::read(&job.output).unwrap(), b"previous");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn commit_keeps_new_output() {
        let (dir, job) = fixture("commit");
        std::fs::write(&job.output, b"previous").unwrap();
        let tx = OutputTransaction::begin(&job, true).unwrap();
        std::fs::write(&job.output, b"verified").unwrap();
        tx.commit();
        assert_eq!(std::fs::read(&job.output).unwrap(), b"verified");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn refusing_overwrite_never_touches_existing_output() {
        let (dir, job) = fixture("no-overwrite");
        std::fs::write(&job.output, b"previous").unwrap();
        assert!(OutputTransaction::begin(&job, false).is_err());
        assert_eq!(std::fs::read(&job.output).unwrap(), b"previous");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn staged_publish_replaces_only_after_success() {
        let (dir, job) = fixture("staged-publish");
        std::fs::write(&job.output, b"previous").unwrap();
        let staged = StagedOutput::new(&job, true).unwrap();

        assert_eq!(std::fs::read(&job.output).unwrap(), b"previous");
        std::fs::write(&staged.execution_job.output, b"verified").unwrap();
        assert_eq!(std::fs::read(&job.output).unwrap(), b"previous");

        staged.publish(&job, true).unwrap();
        assert_eq!(std::fs::read(&job.output).unwrap(), b"verified");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn staged_cleanup_preserves_existing_output() {
        let (dir, job) = fixture("staged-cleanup");
        std::fs::write(&job.output, b"previous").unwrap();
        let staged = StagedOutput::new(&job, true).unwrap();
        std::fs::write(&staged.execution_job.output, b"invalid").unwrap();

        staged.cleanup();
        assert_eq!(std::fs::read(&job.output).unwrap(), b"previous");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn publishes_additional_split_outputs() {
        let (dir, mut job) = fixture("staged-parts");
        job.output = dir.join("game.iso").to_string_lossy().to_string();
        job.mode = "ps3build".into();
        let staged = StagedOutput::new(&job, true).unwrap();
        let stage_output = PathBuf::from(&staged.execution_job.output);
        std::fs::write(stage_output.with_extension("iso.0"), b"part-zero").unwrap();
        std::fs::write(stage_output.with_extension("iso.1"), b"part-one").unwrap();

        staged.publish(&job, true).unwrap();
        assert_eq!(std::fs::read(dir.join("game.iso.0")).unwrap(), b"part-zero");
        assert_eq!(std::fs::read(dir.join("game.iso.1")).unwrap(), b"part-one");
        assert_eq!(output_size_of(&job), 17);
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Tamaño del resultado, sea un archivo suelto o una carpeta entera.
fn output_size_of(job: &Job) -> u64 {
    let p = Path::new(&job.output);
    if mode_writes_directory(job) {
        dir_size(p)
    } else if job.mode == "ps3build" && !p.is_file() {
        split_output_parts(p)
            .iter()
            .filter_map(|part| std::fs::metadata(part).ok())
            .map(|meta| meta.len())
            .sum()
    } else {
        std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
    }
}

fn split_output_parts(output: &Path) -> Vec<PathBuf> {
    let (Some(parent), Some(name)) = (output.parent(), output.file_name()) else {
        return vec![];
    };
    let prefix = format!("{}.", name.to_string_lossy());
    let Ok(entries) = std::fs::read_dir(parent) else {
        return vec![];
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .map(|candidate| candidate.to_string_lossy())
                .and_then(|candidate| candidate.strip_prefix(&prefix).map(str::to_string))
                .map(|suffix| !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false)
        })
        .collect()
}

fn published_size(paths: &[PathBuf], fallback: &Job) -> u64 {
    if paths.is_empty() {
        return output_size_of(fallback);
    }
    paths
        .iter()
        .map(|path| {
            if path.is_dir() {
                dir_size(path)
            } else {
                std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
            }
        })
        .sum()
}

fn display_output(paths: &[PathBuf], fallback: &Job) -> String {
    let expected = Path::new(&fallback.output);
    if expected.exists() || paths.is_empty() {
        return fallback.output.clone();
    }
    if paths.len() == 1 {
        return paths[0].to_string_lossy().to_string();
    }
    expected
        .parent()
        .unwrap_or(expected)
        .to_string_lossy()
        .to_string()
}

/// Varias herramientas escriben en una carpeta, no en un archivo concreto.
fn out_dir_of(job: &Job) -> String {
    Path::new(&job.output)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Construye la linea de argumentos de chdman para un trabajo.
fn chdman_args(job: &Job, s: &Settings) -> Vec<String> {
    let mut a: Vec<String> = vec![job.mode.clone()];
    a.push("-i".into());
    a.push(job.input.clone());

    if job.mode != "verify" {
        a.push("-o".into());
        a.push(job.output.clone());
        if let Some(extra) = &job.output_extra {
            a.push("-ob".into());
            a.push(extra.clone());
        }
        if s.overwrite {
            a.push("-f".into());
        }
    }

    if job.mode.starts_with("create") {
        if !job.codecs.is_empty() {
            a.push("-c".into());
            a.push(job.codecs.join(","));
        }
        if let Some(hs) = job.hunk_size {
            a.push("-hs".into());
            a.push(hs.to_string());
        }
        if job.mode == "createraw" {
            a.push("-us".into());
            a.push(job.unit_size.unwrap_or(512).to_string());
        }
        if s.threads > 0 {
            a.push("-np".into());
            a.push(s.threads.to_string());
        }
    }

    a
}

/// Lee el porcentaje de una linea de progreso.
///
/// chdman escribe "Compressing, 43.2% complete... (ratio=51.0%)" mientras que
/// nsz usa una barra estilo tqdm con el porcentaje suelto, asi que se admiten
/// las dos formas.
fn parse_progress(line: &str) -> Option<(f32, Option<f32>, String)> {
    let pct: f32 = match line.find("% complete") {
        Some(idx) => number_before(&line[..idx])?,
        None => percent_token(line)?,
    };
    if !(0.0..=100.0).contains(&pct) {
        return None;
    }

    let ratio = line.find("ratio=").and_then(|i| {
        line[i + 6..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>()
            .parse::<f32>()
            .ok()
    });

    let head = line.split(',').next().unwrap_or("").trim();
    let phase = if head.contains("Compress") {
        "Comprimiendo"
    } else if head.contains("Extract") || head.contains("Decompress") {
        "Extrayendo"
    } else if head.contains("Verif") {
        "Verificando"
    } else if head.contains("Analyz") {
        "Analizando"
    } else {
        "Procesando"
    };

    Some((pct, ratio, phase.to_string()))
}

/// Numero pegado al final de un fragmento: "Compressing, 43.2" -> 43.2
fn number_before(head: &str) -> Option<f32> {
    let num: String = head
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    num.parse().ok()
}

/// Ultimo token con forma de porcentaje de la linea: "[ 43%|####]" -> 43
fn percent_token(line: &str) -> Option<f32> {
    let mut best = None;
    for (i, c) in line.char_indices() {
        if c == '%' {
            if let Some(v) = number_before(&line[..i]) {
                best = Some(v);
            }
        }
    }
    best
}

/// Lee un pipe partiendo por \r y \n, ya que chdman reescribe la misma linea.
async fn pump_pipe<R>(reader: R, tx: tokio::sync::mpsc::UnboundedSender<String>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(reader);
    let mut buf = [0u8; 1024];
    let mut acc = String::new();
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                acc.push_str(&String::from_utf8_lossy(&buf[..n]));
                while let Some(pos) = acc.find(['\r', '\n']) {
                    let line = acc[..pos].trim().to_string();
                    acc.drain(..pos + 1);
                    if !line.is_empty() {
                        let _ = tx.send(line);
                    }
                }
            }
        }
    }
    let rest = acc.trim().to_string();
    if !rest.is_empty() {
        let _ = tx.send(rest);
    }
}

async fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let mut c = tokio::process::Command::new("taskkill");
        c.args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        chdman::hide_console(&mut c);
        let _ = c.status().await;
    }
    #[cfg(not(windows))]
    {
        let _ = tokio::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .await;
    }
}

/// CIA -> CCI descifrado.
///
/// No basta con sacar el contenido y volver a empaquetarlo: dentro del CIA cada
/// particion es un NCCH con su propio cifrado, y Azahar no sabe descifrar. Hay
/// que deshacerlo aqui, y para eso hacen falta tres herramientas:
///
///   1. `ctrtool --contents`  saca las particiones (aun cifradas)
///   2. `3dstool --header`    copia la cabecera NCCH, que va en claro
///   3. `ctrtool --exefs...`  descifra las secciones con boot9 y la seeddb
///   4. `3dstool -cvtf`       las vuelve a montar marcadas como no cifradas
///   5. `makerom -f cci`      junta las particiones en el CCI final
///
/// Los intermedios se van borrando segun dejan de hacer falta: un juego grande
/// llegaria a ocupar cuatro veces su tamano si se guardaran todos a la vez.
async fn run_cia2cci(app: AppHandle, id: String, job: Job, s: Settings, cancel: Arc<AtomicBool>) {
    let state = app.state::<AppState>();

    let staged = match StagedOutput::new(&job, s.overwrite) {
        Ok(staged) => staged,
        Err(message) => {
            if let Some(j) = state.update(&id, |j| {
                j.status = "error".into();
                j.phase = "No iniciado".into();
                j.message = Some(message.clone());
                j.verification = "not_applicable".into();
                j.verification_message = Some("La conversion no llego a ejecutarse".into());
                j.finished_at = Some(now_ms());
            }) {
                emit_job(&app, &j);
            }
            return;
        }
    };
    let execution_job = &staged.execution_job;

    let fail = |msg: String| {
        staged.cleanup();
        let st = app.state::<AppState>();
        if let Some(j) = st.update(&id, |j| {
            j.status = "error".into();
            j.phase = "Error".into();
            j.message = Some(msg.clone());
            j.verification = "not_applicable".into();
            j.verification_message = Some("La conversion termino con error".into());
            j.finished_at = Some(now_ms());
        }) {
            emit_job(&app, &j);
        }
    };

    let paso = |texto: &str, pct: f32| {
        let st = app.state::<AppState>();
        if let Some(j) = st.update(&id, |j| {
            j.phase = texto.to_string();
            j.progress = pct;
        }) {
            emit_job(&app, &j);
        }
    };

    let canceled = || {
        staged.cleanup();
        let _ = std::fs::remove_dir_all(std::env::temp_dir().join(format!("chd-studio-cia-{id}")));
        let st = app.state::<AppState>();
        if let Some(j) = st.update(&id, |j| {
            j.status = "canceled".into();
            j.phase = "Cancelado".into();
            j.message = Some("Cancelado por el usuario".into());
            j.verification = "not_applicable".into();
            j.verification_message = Some("La conversion o verificacion fue cancelada".into());
            j.output_size = 0;
            j.finished_at = Some(now_ms());
        }) {
            emit_job(&app, &j);
        }
    };

    let Some(ctrtool) = crate::tools::locate("ctrtool", &s).map(|(p, _)| p) else {
        return fail("Falta ctrtool. Instalalo desde Ajustes -> Herramientas.".into());
    };
    let Some(tresdstool) = crate::tools::locate("3dstool", &s).map(|(p, _)| p) else {
        return fail("Falta 3dstool. Instalalo desde Ajustes -> Herramientas.".into());
    };
    let Some(makerom) = crate::tools::locate("makerom", &s).map(|(p, _)| p) else {
        return fail("Falta makerom. Instalalo desde Ajustes -> Herramientas.".into());
    };

    let work = std::env::temp_dir().join(format!("chd-studio-cia-{}", id));
    if let Err(e) = std::fs::create_dir_all(&work) {
        return fail(format!("No se pudo crear la carpeta temporal: {e}"));
    }

    // ctrtool solo lee boot9 de <HOME>/.3ds, asi que se le monta uno temporal
    let mut env: Vec<(&str, String)> = vec![];
    if let Some(home) = crate::threeds::prepare_ctrtool_home(&s, &work) {
        let h = home.to_string_lossy().to_string();
        env.push(("HOME", h.clone()));
        env.push(("USERPROFILE", h));
    }

    paso("Extrayendo el CIA", 5.0);
    let args = crate::threeds::ctrtool_args(&job.input, &work);
    match chdman::run_capture_cancelable_in(&ctrtool, &args, &env, None, cancel.as_ref()).await {
        Ok(chdman::CaptureResult::Canceled) => return canceled(),
        Ok(chdman::CaptureResult::Finished(false, out)) => {
            let _ = std::fs::remove_dir_all(&work);
            let tail = out
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("ctrtool fallo");
            return fail(format!("ctrtool: {tail}"));
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&work);
            return fail(format!("No se pudo ejecutar ctrtool: {e}"));
        }
        Ok(chdman::CaptureResult::Finished(true, _)) => {}
    }

    let partes = crate::threeds::collect_contents(&work);
    if partes.is_empty() {
        let _ = std::fs::remove_dir_all(&work);
        return fail("ctrtool no extrajo ningun contenido del CIA.".into());
    }

    // Cada particion se descifra y se vuelve a montar por separado
    let total = partes.len().max(1);
    let mut descifradas: Vec<(String, u32)> = vec![];

    for (n, (nombre, idx)) in partes.iter().enumerate() {
        let base = 10.0 + (n as f32 / total as f32) * 75.0;
        let tipo = crate::threeds::tipo_particion(*idx);
        let ncch = work.join(nombre);
        let sec = work.join(format!("p{idx}"));
        if let Err(e) = std::fs::create_dir_all(&sec) {
            let _ = std::fs::remove_dir_all(&work);
            return fail(format!("No se pudo crear la carpeta de trabajo: {e}"));
        }

        paso(&format!("Leyendo la particion {}", idx + 1), base);
        let args = crate::threeds::header_args(
            tipo,
            &ncch.to_string_lossy(),
            &sec.join("ncch.bin").to_string_lossy(),
        );
        match chdman::run_capture_cancelable(&tresdstool, &args, cancel.as_ref()).await {
            Ok(chdman::CaptureResult::Canceled) => return canceled(),
            Ok(chdman::CaptureResult::Finished(false, out)) => {
                let _ = std::fs::remove_dir_all(&work);
                let tail = out
                    .lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("");
                return fail(format!("3dstool no pudo leer la cabecera: {tail}"));
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&work);
                return fail(format!("No se pudo ejecutar 3dstool: {e}"));
            }
            Ok(chdman::CaptureResult::Finished(true, _)) => {}
        }

        paso(
            &format!("Descifrando la particion {}", idx + 1),
            base + 20.0 / total as f32,
        );
        let args = crate::threeds::split_args(&ncch.to_string_lossy(), &sec, &s);
        match chdman::run_capture_cancelable_in(&ctrtool, &args, &env, None, cancel.as_ref()).await
        {
            Ok(chdman::CaptureResult::Canceled) => return canceled(),
            Ok(chdman::CaptureResult::Finished(_, out))
                if out.to_lowercase().contains("unable to decrypt") =>
            {
                let _ = std::fs::remove_dir_all(&work);
                let falta_seed = out.to_lowercase().contains("seed");
                return fail(if falta_seed {
                    "Este juego usa cifrado por semilla y no se encontro seeddb.bin. \
                     Indicale su ruta en esta misma pantalla."
                        .into()
                } else {
                    "No se pudo descifrar el contenido. Revisa que boot9.bin sea correcto."
                        .to_string()
                });
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&work);
                return fail(format!("No se pudo descifrar: {e}"));
            }
            Ok(chdman::CaptureResult::Finished(false, out)) => {
                let _ = std::fs::remove_dir_all(&work);
                let tail = out
                    .lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("");
                return fail(format!("ctrtool no pudo descifrar: {tail}"));
            }
            Ok(chdman::CaptureResult::Finished(true, _)) => {}
        }

        // El NCCH cifrado ya no hace falta y ocupa lo mismo que el juego
        let _ = std::fs::remove_file(&ncch);

        paso(
            &format!("Rearmando la particion {}", idx + 1),
            base + 45.0 / total as f32,
        );
        let salida = work.join(format!("dec{idx}.{tipo}"));
        let args = crate::threeds::rebuild_args(tipo, &salida.to_string_lossy(), &sec);
        match chdman::run_capture_cancelable(&tresdstool, &args, cancel.as_ref()).await {
            Ok(chdman::CaptureResult::Canceled) => return canceled(),
            Ok(chdman::CaptureResult::Finished(false, out)) => {
                let _ = std::fs::remove_dir_all(&work);
                let tail = out
                    .lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("");
                return fail(format!("3dstool no pudo rearmar la particion: {tail}"));
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&work);
                return fail(format!("No se pudo rearmar la particion: {e}"));
            }
            Ok(chdman::CaptureResult::Finished(true, _)) => {}
        }

        let _ = std::fs::remove_dir_all(&sec);
        descifradas.push((
            salida.file_name().unwrap().to_string_lossy().to_string(),
            *idx,
        ));
    }

    paso("Montando el CCI", 88.0);
    if let Some(parent) = Path::new(&execution_job.output).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let args = crate::threeds::makerom_args(&execution_job.output, &descifradas);
    let resultado =
        chdman::run_capture_cancelable_in(&makerom, &args, &[], Some(&work), cancel.as_ref()).await;
    let _ = std::fs::remove_dir_all(&work);

    match resultado {
        Ok(chdman::CaptureResult::Canceled) => return canceled(),
        Ok(chdman::CaptureResult::Finished(true, _)) => {}
        Ok(chdman::CaptureResult::Finished(false, out)) => {
            let tail = out
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("makerom fallo");
            return fail(format!("makerom: {tail}"));
        }
        Err(e) => return fail(format!("No se pudo ejecutar makerom: {e}")),
    }

    let out_size = std::fs::metadata(&execution_job.output)
        .map(|m| m.len())
        .unwrap_or(0);
    if out_size == 0 {
        return fail("makerom no genero ningun archivo".into());
    }

    paso("Verificando", 99.0);
    if let Some(j) = state.update(&id, |j| {
        j.verification = "running".into();
        j.verification_message = None;
    }) {
        emit_job(&app, &j);
    }
    let verification =
        match crate::verification::verify(execution_job, &s, &makerom, cancel.as_ref()).await {
            Ok(outcome) => outcome,
            Err(crate::verification::VerifyError::Canceled) => return canceled(),
            Err(crate::verification::VerifyError::Failed(message)) => {
                staged.cleanup();
                if let Some(j) = state.update(&id, |j| {
                    j.status = "error".into();
                    j.phase = "Verificacion fallida".into();
                    j.message = Some(message.clone());
                    j.verification = "failed".into();
                    j.verification_message = Some(message.clone());
                    j.output_size = 0;
                    j.finished_at = Some(now_ms());
                }) {
                    emit_job(&app, &j);
                }
                return;
            }
        };

    let published = match staged.publish(&job, s.overwrite) {
        Ok(paths) => paths,
        Err(message) => {
            staged.cleanup();
            if let Some(j) = state.update(&id, |j| {
                j.status = "error".into();
                j.phase = "No se pudo publicar".into();
                j.message = Some(message.clone());
                j.verification = verification.status.into();
                j.verification_message = Some(verification.message.clone());
                j.output_size = 0;
                j.finished_at = Some(now_ms());
            }) {
                emit_job(&app, &j);
            }
            return;
        }
    };

    let out_size = published_size(&published, &job);
    let visible_output = display_output(&published, &job);

    if let Some(j) = state.update(&id, |j| {
        j.status = "done".into();
        j.phase = if verification.status == "passed" {
            "Listo · verificado".into()
        } else {
            "Listo · validacion basica".into()
        };
        j.progress = 100.0;
        j.output_size = out_size;
        j.output = visible_output.clone();
        j.verification = verification.status.into();
        j.verification_message = Some(verification.message.clone());
        j.finished_at = Some(now_ms());
    }) {
        emit_job(&app, &j);
    }
}

/// Ejecuta un trabajo de principio a fin, emitiendo progreso al frontend.
async fn run_job(app: AppHandle, id: String) {
    let state = app.state::<AppState>();
    let (job, settings, exe) = {
        let s = state.settings.lock().unwrap().clone();
        let job = match state.jobs.lock().unwrap().iter().find(|j| j.id == id) {
            Some(j) => j.clone(),
            None => return,
        };
        // ps3iso-utils son cuatro programas en una sola descarga: hay que coger
        // el que toque segun el modo.
        let exe = if job.tool == "ps3iso" {
            crate::tools::locate_sibling("ps3iso", crate::ps3::exe_for(&job.mode))
        } else {
            crate::tools::locate(&job.tool, &s).map(|(p, _)| p)
        };
        (job, s, exe)
    };

    // Este modo encadena dos herramientas, asi que sigue su propio camino
    if job.mode == "cia2cci" {
        let cancel = Arc::new(AtomicBool::new(false));
        state
            .cancels
            .lock()
            .unwrap()
            .insert(id.clone(), cancel.clone());
        if let Some(j) = state.update(&id, |j| {
            j.status = "running".into();
            j.phase = "Iniciando".into();
            j.started_at = Some(now_ms());
            j.progress = 0.0;
            j.verification = "pending".into();
            j.verification_message = None;
        }) {
            emit_job(&app, &j);
        }
        run_cia2cci(app.clone(), id.clone(), job, settings, cancel).await;
        state.cancels.lock().unwrap().remove(&id);
        return;
    }

    let Some(exe) = exe else {
        let tool = job.tool.clone();
        if let Some(j) = state.update(&id, |j| {
            j.status = "error".into();
            j.message = Some(format!(
                "No se encontro «{tool}». Instalalo desde Ajustes → Herramientas."
            ));
            j.verification = "not_applicable".into();
            j.verification_message = Some("La conversion no llego a ejecutarse".into());
            j.finished_at = Some(now_ms());
        }) {
            emit_job(&app, &j);
        }
        return;
    };

    let cancel = Arc::new(AtomicBool::new(false));
    state
        .cancels
        .lock()
        .unwrap()
        .insert(id.clone(), cancel.clone());

    if let Some(j) = state.update(&id, |j| {
        j.status = "running".into();
        j.phase = "Iniciando".into();
        j.started_at = Some(now_ms());
        j.progress = 0.0;
        j.verification = "pending".into();
        j.verification_message = None;
    }) {
        emit_job(&app, &j);
    }

    let staged = match StagedOutput::new(&job, settings.overwrite) {
        Ok(staged) => staged,
        Err(message) => {
            if let Some(j) = state.update(&id, |j| {
                j.status = "error".into();
                j.phase = "No iniciado".into();
                j.message = Some(message.clone());
                j.verification = "not_applicable".into();
                j.verification_message = Some("La conversion no llego a ejecutarse".into());
                j.finished_at = Some(now_ms());
            }) {
                emit_job(&app, &j);
            }
            state.cancels.lock().unwrap().remove(&id);
            return;
        }
    };

    // Asegura que exista la carpeta de destino. Si la salida es en si una
    // carpeta (GOD), hay que crearla entera y no solo la que la contiene.
    let execution_job = &staged.execution_job;
    if writes_directory(&execution_job.tool) {
        let _ = std::fs::create_dir_all(&execution_job.output);
    } else if let Some(parent) = Path::new(&execution_job.output).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let args = build_args(execution_job, &settings);
    let mut cmd = tokio::process::Command::new(&exe);
    cmd.args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    chdman::hide_console(&mut cmd);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            staged.cleanup();
            if let Some(j) = state.update(&id, |j| {
                j.status = "error".into();
                j.message = Some(format!("No se pudo iniciar chdman: {e}"));
                j.verification = "not_applicable".into();
                j.verification_message = Some("La conversion no llego a ejecutarse".into());
                j.finished_at = Some(now_ms());
            }) {
                emit_job(&app, &j);
            }
            state.cancels.lock().unwrap().remove(&id);
            return;
        }
    };

    if let Some(pid) = child.id() {
        state.children.lock().unwrap().insert(id.clone(), pid);
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    if let Some(out) = child.stdout.take() {
        tauri::async_runtime::spawn(pump_pipe(out, tx.clone()));
    }
    if let Some(err) = child.stderr.take() {
        tauri::async_runtime::spawn(pump_pipe(err, tx.clone()));
    }
    drop(tx);

    // Varias herramientas (maxcso, extract-xiso, las de PS3...) callan cuando
    // su salida va a una tuberia en vez de a una consola, asi que nunca llega
    // un porcentaje. Sin esto el trabajo parece colgado aunque este avanzando.
    // El latido mira cuanto ha escrito ya y lo va contando.
    let saw_progress = Arc::new(AtomicBool::new(false));
    let done_flag = Arc::new(AtomicBool::new(false));
    {
        let app = app.clone();
        let id = id.clone();
        let job_out = execution_job.output.clone();
        let is_dir = mode_writes_directory(execution_job);
        let saw = saw_progress.clone();
        let done = done_flag.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1200)).await;
                if done.load(Ordering::Relaxed) {
                    break;
                }
                if saw.load(Ordering::Relaxed) {
                    continue;
                }
                let p = Path::new(&job_out);
                let size = if is_dir {
                    dir_size(p)
                } else {
                    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
                };
                let st = app.state::<AppState>();
                if let Some(j) = st.update(&id, |j| {
                    j.output_size = size;
                    if j.phase == "Iniciando" {
                        j.phase = "Procesando".into();
                    }
                }) {
                    emit_job(&app, &j);
                }
            }
        });
    }

    let mut last_emit = Instant::now() - Duration::from_secs(1);
    let mut last_pct = -1.0f32;

    while let Some(line) = rx.recv().await {
        if cancel.load(Ordering::Relaxed) {
            break;
        }

        if let Some((pct, ratio, phase)) = parse_progress(&line) {
            saw_progress.store(true, Ordering::Relaxed);
            let changed = (pct - last_pct).abs() >= 0.2;
            if changed && last_emit.elapsed() >= Duration::from_millis(120) {
                last_pct = pct;
                last_emit = Instant::now();
                if let Some(j) = state.update(&id, |j| {
                    j.progress = pct;
                    j.phase = phase.clone();
                    if ratio.is_some() {
                        j.ratio = ratio;
                    }
                }) {
                    emit_job(&app, &j);
                }
            }
        } else {
            let final_ratio = line.find("final ratio =").and_then(|i| {
                line[i + 13..]
                    .trim()
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect::<String>()
                    .parse::<f32>()
                    .ok()
            });
            if let Some(j) = state.update(&id, |j| {
                if let Some(r) = final_ratio {
                    j.ratio = Some(r);
                }
                j.log.push(line.clone());
                if j.log.len() > 400 {
                    j.log.remove(0);
                }
            }) {
                emit_job(&app, &j);
            }
        }
    }

    if cancel.load(Ordering::Relaxed) {
        // El guard del mutex se suelta aqui, antes del await
        let pid = state.children.lock().unwrap().get(&id).copied();
        if let Some(pid) = pid {
            kill_pid(pid).await;
        }
        let _ = child.start_kill();
    }

    done_flag.store(true, Ordering::Relaxed);

    let ok = match child.wait().await {
        Ok(st) => st.success(),
        Err(_) => false,
    };

    state.children.lock().unwrap().remove(&id);

    let canceled = cancel.load(Ordering::Relaxed);

    if canceled {
        // La salida final nunca se toco: basta retirar el area temporal.
        staged.cleanup();
        if let Some(j) = state.update(&id, |j| {
            j.status = "canceled".into();
            j.phase = "Cancelado".into();
            j.message = Some("Cancelado por el usuario".into());
            j.verification = "not_applicable".into();
            j.verification_message = Some("No se verifico un trabajo cancelado".into());
            j.finished_at = Some(now_ms());
        }) {
            emit_job(&app, &j);
        }
        state.cancels.lock().unwrap().remove(&id);
        return;
    }

    if !ok {
        let msg = {
            let jobs = state.jobs.lock().unwrap();
            jobs.iter()
                .find(|j| j.id == id)
                .and_then(|j| {
                    j.log
                        .iter()
                        .rev()
                        .find(|l| {
                            let l = l.to_lowercase();
                            l.contains("error") || l.contains("unable") || l.contains("fatal")
                        })
                        .cloned()
                })
                .unwrap_or_else(|| "chdman termino con error".into())
        };
        staged.cleanup();
        if let Some(j) = state.update(&id, |j| {
            j.status = "error".into();
            j.phase = "Error".into();
            j.message = Some(msg.clone());
            j.verification = "not_applicable".into();
            j.verification_message = Some("La herramienta termino con error".into());
            j.finished_at = Some(now_ms());
        }) {
            emit_job(&app, &j);
        }
        return;
    }

    // Un trabajo solo puede quedar como terminado despues de validar su salida.
    if let Some(j) = state.update(&id, |j| {
        j.phase = "Verificando".into();
        j.progress = 99.0;
        j.verification = "running".into();
        j.verification_message = None;
    }) {
        emit_job(&app, &j);
    }

    let verification =
        match crate::verification::verify(execution_job, &settings, &exe, cancel.as_ref()).await {
            Ok(outcome) => outcome,
            Err(crate::verification::VerifyError::Canceled) => {
                staged.cleanup();
                if let Some(j) = state.update(&id, |j| {
                    j.status = "canceled".into();
                    j.phase = "Cancelado".into();
                    j.message = Some("Cancelado durante la verificacion".into());
                    j.verification = "not_applicable".into();
                    j.verification_message = Some("La verificacion fue cancelada".into());
                    j.output_size = 0;
                    j.finished_at = Some(now_ms());
                }) {
                    emit_job(&app, &j);
                }
                state.cancels.lock().unwrap().remove(&id);
                return;
            }
            Err(crate::verification::VerifyError::Failed(message)) => {
                staged.cleanup();
                if let Some(j) = state.update(&id, |j| {
                    j.status = "error".into();
                    j.phase = "Verificacion fallida".into();
                    j.message = Some(message.clone());
                    j.verification = "failed".into();
                    j.verification_message = Some(message.clone());
                    j.output_size = 0;
                    j.finished_at = Some(now_ms());
                }) {
                    emit_job(&app, &j);
                }
                state.cancels.lock().unwrap().remove(&id);
                return;
            }
        };

    let published = match staged.publish(&job, settings.overwrite) {
        Ok(paths) => paths,
        Err(message) => {
            staged.cleanup();
            if let Some(j) = state.update(&id, |j| {
                j.status = "error".into();
                j.phase = "No se pudo publicar".into();
                j.message = Some(message.clone());
                j.verification = verification.status.into();
                j.verification_message = Some(verification.message.clone());
                j.output_size = 0;
                j.finished_at = Some(now_ms());
            }) {
                emit_job(&app, &j);
            }
            state.cancels.lock().unwrap().remove(&id);
            return;
        }
    };

    let out_size = published_size(&published, &job);
    let visible_output = display_output(&published, &job);

    // Solo se borra el origen si de verdad se genero algo nuevo
    let produced = job.mode.starts_with("create")
        || matches!(
            job.mode.as_str(),
            "nsp2nsz" | "xci2xcz" | "nsz2nsp" | "xcz2xci" | "xci2nsp"
        );
    if settings.delete_source && produced && out_size > 0 {
        delete_source_set(&job.input);
    }

    if let Some(j) = state.update(&id, |j| {
        j.status = "done".into();
        j.phase = if verification.status == "passed" {
            "Listo · verificado".into()
        } else {
            "Listo · validacion basica".into()
        };
        j.progress = 100.0;
        j.output_size = out_size;
        j.output = visible_output.clone();
        j.verification = verification.status.into();
        j.verification_message = Some(verification.message.clone());
        j.finished_at = Some(now_ms());
    }) {
        emit_job(&app, &j);
    }
    state.cancels.lock().unwrap().remove(&id);
}

/// Al borrar el origen de un .cue/.gdi hay que borrar tambien sus pistas.
fn delete_source_set(input: &str) {
    let p = PathBuf::from(input);
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if ext == "cue" || ext == "gdi" || ext == "toc" {
        if let Ok(text) = std::fs::read_to_string(&p) {
            if let Some(dir) = p.parent() {
                for line in text.lines() {
                    // Nombres de pista entre comillas (cue) o sueltos (gdi)
                    let candidates: Vec<String> = if let Some(a) = line.find('"') {
                        line[a + 1..]
                            .find('"')
                            .map(|b| vec![line[a + 1..a + 1 + b].to_string()])
                            .unwrap_or_default()
                    } else {
                        line.split_whitespace()
                            .filter(|t| {
                                let t = t.to_lowercase();
                                t.ends_with(".bin") || t.ends_with(".raw") || t.ends_with(".iso")
                            })
                            .map(|s| s.to_string())
                            .collect()
                    };
                    for c in candidates {
                        let f = dir.join(c);
                        if f.is_file() {
                            let _ = std::fs::remove_file(f);
                        }
                    }
                }
            }
        }
    }
    let _ = std::fs::remove_file(&p);
}

/// Bucle que va sacando trabajos de la cola respetando el limite de paralelismo.
pub fn start_pump(app: AppHandle) {
    let state = app.state::<AppState>();
    if state.pumping.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let state = app.state::<AppState>();
            let parallel = state.settings.lock().unwrap().parallel.max(1);
            let (running, next) = {
                let jobs = state.jobs.lock().unwrap();
                let running = jobs.iter().filter(|j| j.status == "running").count();
                let next = jobs
                    .iter()
                    .find(|j| j.status == "queued")
                    .map(|j| j.id.clone());
                (running, next)
            };
            if running < parallel {
                if let Some(id) = next {
                    // Marcado inmediato para que el siguiente tick no lo tome dos veces
                    let _ = state.update(&id, |j| j.status = "running".into());
                    let app2 = app.clone();
                    tauri::async_runtime::spawn(async move { run_job(app2, id).await });
                }
            }
        }
    });
}

pub fn cancel(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    if let Some(flag) = state.cancels.lock().unwrap().get(id) {
        flag.store(true, Ordering::Relaxed);
    }
    let pid = state.children.lock().unwrap().get(id).copied();
    if let Some(pid) = pid {
        tauri::async_runtime::spawn(async move { kill_pid(pid).await });
    } else if let Some(j) = state.update(id, |j| {
        if j.status == "queued" {
            j.status = "canceled".into();
            j.phase = "Cancelado".into();
        }
    }) {
        emit_job(app, &j);
    }
}
