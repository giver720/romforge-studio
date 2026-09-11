use crate::jobs::Job;
use std::path::Path;

pub const MODE: &str = "ps2fpkg";

pub fn is_mode(mode: &str) -> bool {
    mode == MODE
}

fn option(job: &Job, key: &str) -> Option<String> {
    job.options
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub fn args(job: &Job, output_dir: &Path) -> Result<Vec<String>, String> {
    let extension = Path::new(&job.input)
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if !matches!(extension.as_str(), "iso" | "chd" | "7z" | "zip" | "rar") {
        return Err("El conversor acepta ISO, CHD, 7Z, ZIP o RAR de PS2".into());
    }
    for key in ["lua", "config", "icon", "background"] {
        if let Some(path) = option(job, key) {
            if !Path::new(&path).is_file() {
                return Err(format!(
                    "El archivo configurado en {key} ya no existe: {path}"
                ));
            }
        }
    }

    let mut args = vec![
        job.input.clone(),
        "--out".into(),
        output_dir.to_string_lossy().to_string(),
    ];
    for (key, flag) in [
        ("emu", "--emu"),
        ("title", "--title"),
        ("uprender", "--uprender"),
        ("upscale", "--upscale"),
        ("display_mode", "--display-mode"),
        ("multitap", "--multitap"),
        ("lua", "--lua"),
        ("config", "--config"),
        ("icon", "--icon"),
        ("background", "--bg"),
    ] {
        if let Some(value) = option(job, key) {
            args.extend([flag.into(), value]);
        }
    }
    if job.options.get("auto_art").map(String::as_str) != Some("false") {
        args.push("--auto-art".into());
    }
    if let Some(values) = option(job, "sets") {
        for value in values
            .lines()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if !value.contains('=') || value.starts_with('-') {
                return Err(format!(
                    "Opción avanzada inválida: {value}. Usa nombre=valor."
                ));
            }
            args.extend(["--set".into(), value.into()]);
        }
    }
    args.push("--dump-config".into());
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_safe_configurable_arguments() {
        let mut job = Job::new(
            "game.iso".into(),
            "game.pkg".into(),
            "ps2fpkg".into(),
            MODE.into(),
            "ps2fpkg".into(),
        );
        job.options.insert("emu".into(), "Rogue v1".into());
        job.options.insert("uprender".into(), "2x2".into());
        job.options
            .insert("sets".into(), "host-audio=1\nfoo=bar".into());
        let args = args(&job, Path::new("out")).unwrap();
        assert!(args.windows(2).any(|pair| pair == ["--emu", "Rogue v1"]));
        assert_eq!(args.iter().filter(|value| *value == "--set").count(), 2);
        assert!(args.contains(&"--auto-art".to_string()));
    }

    #[test]
    fn rejects_command_like_custom_flags() {
        let mut job = Job::new(
            "game.iso".into(),
            "game.pkg".into(),
            "ps2fpkg".into(),
            MODE.into(),
            "ps2fpkg".into(),
        );
        job.options.insert("sets".into(), "--bad".into());
        assert!(args(&job, Path::new("out")).is_err());
    }
}
