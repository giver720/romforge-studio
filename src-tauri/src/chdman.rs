use crate::settings;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const EXE_NAME: &str = if cfg!(windows) {
    "chdman.exe"
} else {
    "chdman"
};

/// Aplica la bandera que evita que se abra una consola negra en Windows.
pub fn hide_console(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        // tokio::process::Command expone creation_flags directamente en Windows
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ChdmanStatus {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    /// De donde salio: "manual" | "app" | "path" | "mame"
    pub source: Option<String>,
    /// true si chdman soporta el codec zstd (cdzs / zstd), MAME 0.255+
    pub supports_zstd: bool,
}

/// Carpetas donde puede vivir el chdman que viaja dentro del instalador.
/// En desarrollo Tauri copia `binaries/*` junto al ejecutable; en la version
/// instalada por NSIS quedan bajo `resources\binaries`.
pub fn bundled_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            dirs.push(p.join("binaries"));
            dirs.push(p.join("resources").join("binaries"));
            dirs.push(p.join("resources"));
            dirs.push(p.to_path_buf());
        }
    }
    dirs
}

fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = vec![settings::bin_dir()];
    dirs.extend(bundled_dirs());

    // Junto al ejecutable de la app
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            dirs.push(p.join("bin"));
        }
    }

    // Instalaciones tipicas de MAME / emuladores en Windows
    #[cfg(windows)]
    {
        let roots = ["C:\\", "D:\\", "E:\\"];
        let names = [
            "mame",
            "MAME",
            "Emuladores\\mame",
            "Emulators\\mame",
            "Program Files\\mame",
            "Program Files (x86)\\mame",
            "RetroArch\\system",
            "LaunchBox\\ThirdParty\\chdman",
        ];
        for r in roots {
            for n in names {
                dirs.push(PathBuf::from(format!("{r}{n}")));
            }
        }
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join("mame"));
            dirs.push(home.join("Downloads"));
            dirs.push(home.join("Documents").join("mame"));
        }
    }

    // En Linux chdman llega dentro del paquete mame-tools. `which` ya cubre lo
    // que este en el PATH; esto es para los sitios donde queda fuera de el.
    #[cfg(not(windows))]
    {
        for d in [
            "/usr/games",
            "/usr/local/bin",
            "/usr/lib/mame",
            "/opt/mame",
            "/var/lib/flatpak/exports/bin",
        ] {
            dirs.push(PathBuf::from(d));
        }
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".local").join("bin"));
            dirs.push(home.join("mame"));
            dirs.push(home.join("Descargas"));
            dirs.push(home.join("Downloads"));
        }
    }

    dirs
}

/// Busca chdman: ruta manual -> carpeta de la app -> PATH -> instalaciones de MAME.
pub fn locate(manual: Option<&str>) -> Option<(PathBuf, String)> {
    if let Some(m) = manual {
        let p = PathBuf::from(m);
        if p.is_file() {
            return Some((p, "manual".into()));
        }
    }

    let own = settings::bin_dir().join(EXE_NAME);
    if own.is_file() {
        return Some((own, "app".into()));
    }

    // El que viene dentro del instalador manda sobre lo que haya en el sistema
    for d in bundled_dirs() {
        let c = d.join(EXE_NAME);
        if c.is_file() {
            return Some((c, "bundled".into()));
        }
    }

    if let Ok(p) = which::which("chdman") {
        return Some((p, "path".into()));
    }

    for d in candidate_dirs() {
        let c = d.join(EXE_NAME);
        if c.is_file() {
            return Some((c, "mame".into()));
        }
    }

    None
}

/// Ejecuta chdman sin argumentos para leer el banner de version.
pub async fn probe(path: &Path) -> ChdmanStatus {
    let mut cmd = tokio::process::Command::new(path);
    cmd.arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);

    let (version, help) = match cmd.output().await {
        Ok(out) => {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let v = text
                .lines()
                .find(|l| l.to_lowercase().contains("chdman"))
                .map(|l| l.trim().to_string());
            (v, text)
        }
        Err(e) => (Some(format!("no se pudo ejecutar: {e}")), String::new()),
    };

    ChdmanStatus {
        found: true,
        path: Some(path.to_string_lossy().to_string()),
        supports_zstd: help.contains("cdzs")
            || help.contains("zstd")
            || version.as_deref().map(mame_has_zstd).unwrap_or(false),
        version,
        source: None,
    }
}

/// El codec zstd entro en MAME 0.255; antes solo estaban lzma/zlib/huff/flac.
///
/// El banner es del estilo "chdman - MAME Compressed Hunks of Data (CHD)
/// manager 0.289 (mame0289)", asi que buscamos el token con forma de version.
fn mame_has_zstd(banner: &str) -> bool {
    banner
        .split_whitespace()
        .filter_map(|t| {
            t.trim_matches(|c: char| !c.is_ascii_digit() && c != '.')
                .parse::<f32>()
                .ok()
        })
        .find(|v| *v > 0.0 && *v < 100.0)
        .map(|v| v >= 0.255)
        .unwrap_or(false)
}

pub async fn status(manual: Option<&str>) -> ChdmanStatus {
    match locate(manual) {
        Some((p, src)) => {
            let mut st = probe(&p).await;
            st.source = Some(src);
            st
        }
        None => ChdmanStatus {
            found: false,
            path: None,
            version: None,
            source: None,
            supports_zstd: false,
        },
    }
}

/// Copia un chdman elegido por el usuario a la carpeta privada de la app.
pub fn install_copy(src: &str) -> anyhow::Result<String> {
    let src = PathBuf::from(src);
    anyhow::ensure!(src.is_file(), "El archivo no existe");
    let dir = settings::bin_dir();
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(EXE_NAME);
    std::fs::copy(&src, &dest)?;
    Ok(dest.to_string_lossy().to_string())
}

/// Ejecuta chdman y devuelve toda su salida (para `info` y `verify` puntuales).
pub async fn run_capture(exe: &Path, args: &[String]) -> anyhow::Result<(bool, String)> {
    run_capture_env(exe, args, &[]).await
}

/// Ejecuta una herramienta con limite de tiempo y la mata si se pasa.
///
/// Los sondeos de version tienen que ser inofensivos: algunas herramientas
/// empaquetadas con PyInstaller dejan un proceso hijo que mantiene abiertas las
/// tuberias, y sin esto la espera no termina nunca y se van acumulando procesos.
pub async fn run_capture_timeout(
    exe: &Path,
    args: &[String],
    secs: u64,
) -> anyhow::Result<(bool, String)> {
    let mut cmd = tokio::process::Command::new(exe);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    hide_console(&mut cmd);

    let child = cmd.spawn()?;
    let pid = child.id();

    match tokio::time::timeout(
        std::time::Duration::from_secs(secs),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(out)) => Ok((
            out.status.success(),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )),
        Ok(Err(e)) => Err(e.into()),
        Err(_) => {
            if let Some(pid) = pid {
                kill_stray(pid).await;
            }
            anyhow::bail!("la herramienta no respondio en {secs}s")
        }
    }
}

/// Mata un proceso y su descendencia, que es lo que deja PyInstaller.
async fn kill_stray(pid: u32) {
    #[cfg(windows)]
    {
        let mut c = tokio::process::Command::new("taskkill");
        c.args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_console(&mut c);
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

/// Igual que `run_capture` pero permite fijar variables de entorno, que es como
/// se le indica a ctrtool donde estan sus claves.
pub async fn run_capture_env(
    exe: &Path,
    args: &[String],
    env: &[(&str, String)],
) -> anyhow::Result<(bool, String)> {
    run_capture_in(exe, args, env, None).await
}

/// Como `run_capture_env` pero ademas permite fijar el directorio de trabajo,
/// que hace falta para poder pasarle rutas relativas a makerom.
pub async fn run_capture_in(
    exe: &Path,
    args: &[String],
    env: &[(&str, String)],
    cwd: Option<&Path>,
) -> anyhow::Result<(bool, String)> {
    let mut cmd = tokio::process::Command::new(exe);
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    // stdin cerrado a proposito: varias de estas herramientas piden datos por
    // teclado si no entienden los argumentos, y sin esto se quedan colgadas
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    hide_console(&mut cmd);
    let out = cmd.output().await?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Ok((out.status.success(), text))
}
