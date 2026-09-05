mod artwork;
mod chdman;
mod jobs;
mod ps3;
mod ps5;
mod psp;
mod settings;
mod store;
mod switch;
mod threeds;
mod tools;
mod verification;
mod wii;
mod xbox360;

use jobs::{AppState, Job};
use serde::{Deserialize, Serialize};
use settings::Settings;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager, State};

/// Extensiones que chdman acepta como entrada para crear un CHD.
const CD_EXT: &[&str] = &["cue", "gdi", "toc", "nrg", "cdr"];
const HD_EXT: &[&str] = &["img", "hdi", "vhd", "hdd", "raw"];
/// Formatos habituales que chdman NO sabe leer; los avisamos en vez de fallar en silencio.
const UNSUPPORTED_EXT: &[&str] = &[
    "cdi", "mdf", "mds", "ccd", "rvz", "wbfs", "nkit", "7z", "zip", "rar",
];

#[derive(Debug, Clone, Serialize)]
pub struct InputInfo {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    /// ok | needs_cue | unsupported | missing
    pub state: String,
    /// createcd | createdvd | createhd | chd | ""
    pub suggested_mode: String,
    pub note: Option<String>,
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn stem_of(p: &Path) -> String {
    p.file_stem()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| "salida".into())
}

fn classify(path: &Path) -> InputInfo {
    let ext = ext_of(path);
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let mut info = InputInfo {
        path: path.to_string_lossy().to_string(),
        name,
        ext: ext.clone(),
        size,
        state: "ok".into(),
        suggested_mode: String::new(),
        note: None,
    };

    if !path.is_file() {
        info.state = "missing".into();
        return info;
    }

    if ext == "chd" {
        info.suggested_mode = "chd".into();
        return info;
    }
    if switch::is_switch_ext(&ext) {
        info.suggested_mode = switch::suggest_mode(&ext).unwrap_or("").into();
        return info;
    }
    if threeds::is_3ds_ext(&ext) {
        info.suggested_mode = threeds::suggest_mode(&ext).into();
        return info;
    }
    // El .iso lo reclaman varios modulos, asi que aqui solo entran los
    // comprimidos, que si son inconfundibles de cada consola
    if psp::is_psp_ext(&ext) && ext != "iso" {
        info.suggested_mode = psp::suggest_mode(&ext).into();
        return info;
    }
    if wii::is_wii_ext(&ext) && ext != "iso" {
        info.suggested_mode = wii::suggest_mode(&ext).into();
        return info;
    }
    if CD_EXT.contains(&ext.as_str()) {
        info.suggested_mode = "createcd".into();
        return info;
    }
    if ext == "iso" {
        // Un CD cabe en ~900 MB; por encima casi siempre es un DVD (PS2, Xbox, PSP...)
        info.suggested_mode = if size > 900 * 1024 * 1024 {
            "createdvd".into()
        } else {
            "createcd".into()
        };
        return info;
    }
    if HD_EXT.contains(&ext.as_str()) {
        info.suggested_mode = "createhd".into();
        return info;
    }
    if ext == "bin" {
        // chdman necesita la hoja de ruta (.cue), no el .bin suelto
        let cue = path.with_extension("cue");
        if cue.is_file() {
            info.state = "needs_cue".into();
            info.note = Some(format!("Usa {} en su lugar", stem_of(path) + ".cue"));
        } else {
            info.state = "needs_cue".into();
            info.note = Some("Falta el archivo .cue que describe las pistas".into());
        }
        return info;
    }
    if UNSUPPORTED_EXT.contains(&ext.as_str()) {
        info.state = "unsupported".into();
        info.note = Some(match ext.as_str() {
            "cdi" => "chdman no lee CDI. Convierte antes a CUE/BIN.".into(),
            "mdf" | "mds" => "Formato Alcohol 120%. Convierte antes a CUE/BIN.".into(),
            "ccd" => "CloneCD. Usa el .cue equivalente.".into(),
            "rvz" | "wbfs" | "nkit" => "Formato de GameCube/Wii. Usa Dolphin, no CHD.".into(),
            _ => "Descomprime el archivo primero.".into(),
        });
        return info;
    }

    info.state = "unsupported".into();
    info.note = Some("Extension no reconocida".into());
    info
}

fn walk(path: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    if out.len() > 4000 || depth > 6 {
        return;
    }
    if path.is_file() {
        out.push(path.to_path_buf());
        return;
    }
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            walk(&e.path(), out, depth + 1);
        }
    }
}

#[tauri::command]
fn inspect_paths(paths: Vec<String>) -> Vec<InputInfo> {
    let mut files: Vec<PathBuf> = vec![];
    for p in paths {
        walk(Path::new(&p), &mut files, 0);
    }

    let mut infos: Vec<InputInfo> = files.iter().map(|p| classify(p)).collect();

    // Si un .cue ya cubre un .bin del mismo nombre, quitamos el .bin de la lista
    let cue_stems: std::collections::HashSet<String> = infos
        .iter()
        .filter(|i| CD_EXT.contains(&i.ext.as_str()))
        .map(|i| {
            Path::new(&i.path)
                .with_extension("")
                .to_string_lossy()
                .to_string()
        })
        .collect();
    infos.retain(|i| {
        if i.ext != "bin" {
            return true;
        }
        !cue_stems.contains(
            &Path::new(&i.path)
                .with_extension("")
                .to_string_lossy()
                .to_string(),
        )
    });

    infos.retain(|i| i.state != "unsupported" || !i.ext.is_empty());
    infos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    infos
}

#[derive(Debug, Deserialize)]
pub struct JobSpec {
    pub input: String,
    pub mode: String,
    pub system: String,
    #[serde(default)]
    pub codecs: Vec<String>,
    #[serde(default)]
    pub hunk_size: Option<u32>,
    #[serde(default)]
    pub unit_size: Option<u32>,
    /// Solo para extractcd: cue | gdi | toc
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub output_dir: Option<String>,
}

fn output_for(spec: &JobSpec, s: &Settings) -> (String, Option<String>) {
    let input = PathBuf::from(&spec.input);
    let dir = spec
        .output_dir
        .clone()
        .or_else(|| s.output_dir.clone())
        .map(PathBuf::from)
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or_else(|| input.parent().map(|p| p.to_path_buf()).unwrap_or_default());
    let stem = stem_of(&input);

    // GOD y la extraccion dan carpeta; reconstruir da un ISO
    if xbox360::is_mode(&spec.mode) {
        if spec.mode == "folder2iso" {
            return (
                dir.join(format!("{stem}.iso"))
                    .to_string_lossy()
                    .to_string(),
                None,
            );
        }
        return (dir.join(&stem).to_string_lossy().to_string(), None);
    }

    if wii::is_mode(&spec.mode) {
        let e = wii::output_ext(&spec.mode).unwrap_or("rvz");
        if e.is_empty() {
            return (spec.input.clone(), None);
        }
        return (
            dir.join(format!("{stem}.{e}"))
                .to_string_lossy()
                .to_string(),
            None,
        );
    }

    if let Some(e) = psp::output_ext(&spec.mode) {
        return (
            dir.join(format!("{stem}.{e}"))
                .to_string_lossy()
                .to_string(),
            None,
        );
    }

    // PS3: extraer da una carpeta, reconstruir da un ISO, partir no toca nada
    if ps3::is_mode(&spec.mode) {
        return match spec.mode.as_str() {
            "ps3build" => (
                dir.join(format!("{stem}.iso"))
                    .to_string_lossy()
                    .to_string(),
                None,
            ),
            "ps3compact" => (
                dir.join(format!("{stem}.compact.iso"))
                    .to_string_lossy()
                    .to_string(),
                None,
            ),
            "ps3rpcs3" => (spec.input.clone(), None),
            "ps3split" => (spec.input.clone(), None),
            _ => (dir.join(&stem).to_string_lossy().to_string(), None),
        };
    }

    if ps5::is_mode(&spec.mode) {
        if ps5::writes_directory(&spec.mode) {
            return (
                dir.join(format!("{stem}-extraido"))
                    .to_string_lossy()
                    .to_string(),
                None,
            );
        }
        let ext = ps5::output_ext(&spec.mode).unwrap_or("exfat");
        return (
            dir.join(format!("{stem}.{ext}"))
                .to_string_lossy()
                .to_string(),
            None,
        );
    }

    // Los modos de 3DS necesitan saber la extension de entrada para elegir la de salida
    if threeds::is_mode(&spec.mode) {
        let in_ext = input
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let ext = threeds::output_ext(&spec.mode, &in_ext).unwrap_or("cci");
        return (
            dir.join(format!("{stem}.{ext}"))
                .to_string_lossy()
                .to_string(),
            None,
        );
    }

    // Los modos de Switch declaran su extension de salida en su propio modulo
    if let Some(e) = switch::output_ext(&spec.mode) {
        let ext = if e.is_empty() { "log" } else { e };
        return (
            dir.join(format!("{stem}.{ext}"))
                .to_string_lossy()
                .to_string(),
            None,
        );
    }

    let (ext, extra): (String, Option<String>) = match spec.mode.as_str() {
        "extractcd" => {
            // El .cue/.gdi describe las pistas y el .bin lleva los datos
            let f = spec.format.clone().unwrap_or_else(|| "cue".into());
            let bin = dir
                .join(format!("{stem}.bin"))
                .to_string_lossy()
                .to_string();
            (f, Some(bin))
        }
        "extractdvd" => ("iso".into(), None),
        "extracthd" => ("img".into(), None),
        "extractraw" => ("bin".into(), None),
        _ => ("chd".into(), None),
    };

    (
        dir.join(format!("{stem}.{ext}"))
            .to_string_lossy()
            .to_string(),
        extra,
    )
}

#[tauri::command]
fn add_jobs(app: AppHandle, state: State<AppState>, specs: Vec<JobSpec>) -> Vec<Job> {
    let s = state.settings.lock().unwrap().clone();
    let mut created = vec![];

    for spec in specs {
        // Cada modo sabe que herramienta lo ejecuta
        let tool = switch::tool_for(&spec.mode)
            .or_else(|| threeds::tool_for(&spec.mode))
            .or_else(|| xbox360::tool_for(&spec.mode))
            .or_else(|| ps3::tool_for(&spec.mode))
            .or_else(|| ps5::tool_for_input(&spec.mode, &spec.input))
            .or_else(|| psp::is_mode(&spec.mode).then_some("maxcso"))
            .or_else(|| wii::is_mode(&spec.mode).then_some(wii::tool_for(&spec.mode)))
            .unwrap_or("chdman")
            .to_string();

        if spec.mode == "verify" {
            let mut job = Job::new(
                spec.input.clone(),
                spec.input.clone(),
                tool,
                spec.mode.clone(),
                spec.system.clone(),
            );
            job.phase = "En cola".into();
            created.push(job);
            continue;
        }

        let (output, extra) = output_for(&spec, &s);
        let mut job = Job::new(
            spec.input.clone(),
            output,
            tool,
            spec.mode.clone(),
            spec.system.clone(),
        );
        job.output_extra = extra;
        job.codecs = spec.codecs;
        job.hunk_size = spec.hunk_size;
        job.unit_size = spec.unit_size;
        created.push(job);
    }

    {
        let mut jobs = state.jobs.lock().unwrap();
        for j in &created {
            jobs.push(j.clone());
        }
    }
    let _ = app.emit("jobs://reset", state.snapshot());
    jobs::start_pump(app.clone());
    created
}

#[tauri::command]
fn list_jobs(state: State<AppState>) -> Vec<Job> {
    state.snapshot()
}

#[tauri::command]
fn cancel_job(app: AppHandle, id: String) {
    jobs::cancel(&app, &id);
}

#[tauri::command]
fn remove_job(app: AppHandle, state: State<AppState>, id: String) -> Vec<Job> {
    jobs::cancel(&app, &id);
    state.jobs.lock().unwrap().retain(|j| j.id != id);
    let snap = state.snapshot();
    let _ = app.emit("jobs://reset", snap.clone());
    snap
}

#[tauri::command]
fn retry_job(app: AppHandle, state: State<AppState>, id: String) -> Vec<Job> {
    let _ = state.update(&id, |j| {
        j.status = "queued".into();
        j.phase = "En cola".into();
        j.progress = 0.0;
        j.message = None;
        j.verification = "pending".into();
        j.verification_message = None;
        j.log.clear();
        j.started_at = None;
        j.finished_at = None;
    });
    let snap = state.snapshot();
    let _ = app.emit("jobs://reset", snap.clone());
    jobs::start_pump(app.clone());
    snap
}

#[tauri::command]
fn clear_finished(app: AppHandle, state: State<AppState>) -> Vec<Job> {
    state
        .jobs
        .lock()
        .unwrap()
        .retain(|j| j.status == "queued" || j.status == "running");
    let snap = state.snapshot();
    let _ = app.emit("jobs://reset", snap.clone());
    snap
}

#[tauri::command]
fn cancel_all(app: AppHandle, state: State<AppState>) {
    let ids: Vec<String> = state
        .jobs
        .lock()
        .unwrap()
        .iter()
        .filter(|j| j.status == "queued" || j.status == "running")
        .map(|j| j.id.clone())
        .collect();
    for id in ids {
        jobs::cancel(&app, &id);
    }
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(state: State<AppState>, value: Settings) -> Settings {
    let mut s = state.settings.lock().unwrap();
    *s = value;
    let _ = settings::save(&s);
    s.clone()
}

#[tauri::command]
async fn chdman_status(state: State<'_, AppState>) -> Result<chdman::ChdmanStatus, String> {
    let manual = state.settings.lock().unwrap().chdman_path.clone();
    Ok(chdman::status(manual.as_deref()).await)
}

#[tauri::command]
async fn install_chdman(
    state: State<'_, AppState>,
    path: String,
) -> Result<chdman::ChdmanStatus, String> {
    let dest = chdman::install_copy(&path).map_err(|e| e.to_string())?;
    {
        let mut s = state.settings.lock().unwrap();
        s.chdman_path = Some(dest.clone());
        let _ = settings::save(&s);
    }
    Ok(chdman::status(Some(&dest)).await)
}

#[tauri::command]
async fn chd_info(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let manual = state.settings.lock().unwrap().chdman_path.clone();
    let exe = chdman::locate(manual.as_deref())
        .map(|(p, _)| p)
        .ok_or("No se encontro chdman")?;
    let args = vec!["info".to_string(), "-i".to_string(), path, "-v".to_string()];
    let (_, text) = chdman::run_capture(&exe, &args)
        .await
        .map_err(|e| e.to_string())?;
    Ok(text)
}

#[tauri::command]
async fn tools_status(state: State<'_, AppState>) -> Result<Vec<tools::ToolStatus>, String> {
    let s = state.settings.lock().unwrap().clone();
    Ok(tools::status_all(&s).await)
}

#[tauri::command]
async fn install_tool(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<tools::ToolStatus>, String> {
    tools::install(&id).await.map_err(|e| e.to_string())?;
    let s = state.settings.lock().unwrap().clone();
    let all = tools::status_all(&s).await;
    let _ = app.emit("tools://reset", all.clone());
    Ok(all)
}

#[tauri::command]
async fn set_tool_path(
    state: State<'_, AppState>,
    id: String,
    path: Option<String>,
) -> Result<Vec<tools::ToolStatus>, String> {
    let s = {
        let mut s = state.settings.lock().unwrap();
        match path {
            Some(p) if !p.is_empty() => {
                s.tool_paths.insert(id, p);
            }
            _ => {
                s.tool_paths.remove(&id);
            }
        }
        let _ = settings::save(&s);
        s.clone()
    };
    Ok(tools::status_all(&s).await)
}

#[tauri::command]
async fn python_status() -> Result<tools::PythonStatus, String> {
    Ok(tools::python_status().await)
}

#[tauri::command]
fn switch_keys_status(state: State<AppState>) -> switch::KeysStatus {
    let s = state.settings.lock().unwrap();
    switch::keys_status(&s)
}

/// Lee la ficha del juego con `--dry-run`, sin convertir nada.
/// Sirve para comprobar que el ISO es valido antes de una conversion larga.
#[tauri::command]
async fn xbox_probe(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let s = state.settings.lock().unwrap().clone();
    let exe = tools::locate("iso2god", &s)
        .map(|(p, _)| p)
        .ok_or("Falta iso2god. Instalalo desde Ajustes → Herramientas.")?;

    let (ok, text) = chdman::run_capture(&exe, &xbox360::probe_args(&path))
        .await
        .map_err(|e| e.to_string())?;

    if !ok && text.trim().is_empty() {
        return Err("iso2god no pudo leer ese archivo".into());
    }
    Ok(text)
}

#[tauri::command]
fn threeds_keys_status(state: State<AppState>) -> threeds::KeysStatus {
    let s = state.settings.lock().unwrap();
    threeds::keys_status(&s)
}

/// Lee una carpeta de juego de PS3 extraida y clasifica lo que hay dentro.
#[tauri::command]
fn ps3_scan(dir: String) -> ps3::Ps3Scan {
    ps3::scan(&dir)
}

/// Borra lo seleccionado. Se niega con lo protegido y con rutas de fuera.
#[tauri::command]
fn ps3_trim(dir: String, paths: Vec<String>) -> Result<ps3::TrimResult, String> {
    ps3::trim(&dir, &paths).map_err(|e| e.to_string())
}

#[tauri::command]
fn ps5_scan(dir: String) -> ps5::Ps5Scan {
    ps5::scan(&dir)
}

#[tauri::command]
async fn game_artwork(
    input: String,
    system: String,
    state: State<'_, AppState>,
) -> Result<artwork::GameArtwork, String> {
    let online = state.settings.lock().unwrap().online_artwork;
    Ok(artwork::resolve(&input, &system, online).await)
}

#[derive(Serialize)]
pub struct AppPaths {
    /// true si se esta ejecutando la version portable
    pub portable: bool,
    /// Donde se guardan ajustes, herramientas y el entorno de Python
    pub config_dir: String,
    /// true si esta copia se puede actualizar sola.
    pub can_update: bool,
    /// Como actualizarla cuando no puede hacerlo sola.
    pub update_hint: Option<String>,
}

/// El actualizador de Tauri solo sabe reemplazarse a si mismo en Windows y, en
/// Linux, dentro de un AppImage. Si la app se instalo con .deb o .rpm es el
/// gestor de paquetes el que manda, y lo honesto es decirlo en vez de dejar que
/// el boton falle.
fn updater_disponible() -> (bool, Option<String>) {
    if cfg!(windows) {
        return (true, None);
    }
    if std::env::var_os("APPIMAGE").is_some() {
        (true, None)
    } else {
        (
            false,
            Some(
                "Esta copia se instalo con el paquete del sistema, asi que se actualiza desde ahi \
                 o descargando la nueva version de las releases de GitHub."
                    .into(),
            ),
        )
    }
}

#[tauri::command]
fn app_paths() -> AppPaths {
    let (can_update, update_hint) = updater_disponible();
    AppPaths {
        portable: settings::is_portable(),
        config_dir: settings::config_dir().to_string_lossy().to_string(),
        can_update,
        update_hint,
    }
}

#[tauri::command]
fn reveal(path: String) {
    let p = PathBuf::from(&path);
    let target = if p.is_file() {
        p.parent().map(|x| x.to_path_buf()).unwrap_or(p.clone())
    } else {
        p.clone()
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer")
            .arg(if p.is_file() {
                format!("/select,{}", p.display())
            } else {
                target.display().to_string()
            })
            .creation_flags(chdman::CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        // El equivalente a «mostrar en la carpeta» en Linux es pedirselo al
        // gestor de archivos por D-Bus; asi queda el archivo seleccionado igual
        // que en Windows. Si no hay nadie escuchando se abre la carpeta y ya.
        let seleccionado = p.is_file()
            && std::process::Command::new("dbus-send")
                .args([
                    "--session",
                    "--dest=org.freedesktop.FileManager1",
                    "--type=method_call",
                    "/org/freedesktop/FileManager1",
                    "org.freedesktop.FileManager1.ShowItems",
                    &format!("array:string:file://{}", p.display()),
                    "string:",
                ])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);

        if !seleccionado {
            let _ = std::process::Command::new("xdg-open").arg(target).spawn();
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
                app.handle().plugin(tauri_plugin_process::init())?;
            }
            let state = AppState {
                settings: std::sync::Mutex::new(settings::load()),
                ..Default::default()
            };
            app.manage(state);
            jobs::start_pump(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            inspect_paths,
            add_jobs,
            list_jobs,
            cancel_job,
            cancel_all,
            remove_job,
            retry_job,
            clear_finished,
            get_settings,
            set_settings,
            store::download_homebrew,
            store::fetch_store_catalog,
            store::cancel_store_download,
            store::download_hbas_package,
            chdman_status,
            install_chdman,
            chd_info,
            tools_status,
            install_tool,
            set_tool_path,
            python_status,
            switch_keys_status,
            threeds_keys_status,
            xbox_probe,
            ps3_scan,
            ps3_trim,
            ps5_scan,
            game_artwork,
            app_paths,
            reveal
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar ROMForge Studio");
}
