use crate::jobs::Job;
use std::path::Path;

pub const MODE: &str = "pspfpkg";

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

pub fn args(job: &Job, output: &Path) -> Result<Vec<String>, String> {
    let extension = Path::new(&job.input)
        .extension()
        .map(|value| value.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if extension != "iso" {
        return Err("El creador PSP → PS4 acepta imágenes ISO de PSP".into());
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
    for key in ["textures", "emulator_dir"] {
        if let Some(path) = option(job, key) {
            if !Path::new(&path).is_dir() {
                return Err(format!(
                    "La carpeta configurada en {key} ya no existe: {path}"
                ));
            }
        }
    }

    let mut args = vec![
        "convert".into(),
        "--input".into(),
        job.input.clone(),
        "--output".into(),
        output.to_string_lossy().to_string(),
    ];
    for (key, flag) in [
        ("title", "--title"),
        ("title_id", "--title-id"),
        ("antialias", "--antialias"),
        ("xobuttonmode", "--xobuttonmode"),
        ("lang", "--lang"),
        ("loglevel", "--loglevel"),
        ("securesaves", "--securesaves"),
        ("icon", "--icon"),
        ("background", "--background"),
        ("lua", "--lua"),
        ("config", "--config"),
        ("textures", "--textures"),
        ("emulator_dir", "--emulator-dir"),
    ] {
        if let Some(value) = option(job, key) {
            args.extend([flag.into(), value]);
        }
    }
    for (key, flag, fallback) in [
        ("multisaves", "--multisaves", "true"),
        ("texture_replacement", "--texture-replacement", "false"),
        ("skip_eboot_decrypt", "--skip-eboot-decrypt", "false"),
    ] {
        args.extend([
            flag.into(),
            option(job, key).unwrap_or_else(|| fallback.into()),
        ]);
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
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_safe_psphd_arguments() {
        let mut job = Job::new(
            "game.iso".into(),
            "game.pkg".into(),
            "pspfpkg".into(),
            MODE.into(),
            "psp".into(),
        );
        job.options.insert("antialias".into(), "SSAA4x".into());
        job.options
            .insert("sets".into(), "fast-forward=0\nfoo=bar".into());
        let args = args(&job, Path::new("out.pkg")).unwrap();
        assert!(args
            .windows(2)
            .any(|pair| pair == ["--antialias", "SSAA4x"]));
        assert_eq!(args.iter().filter(|value| *value == "--set").count(), 2);
    }

    #[test]
    fn rejects_command_like_settings() {
        let mut job = Job::new(
            "game.iso".into(),
            "game.pkg".into(),
            "pspfpkg".into(),
            MODE.into(),
            "psp".into(),
        );
        job.options.insert("sets".into(), "--bad".into());
        assert!(args(&job, Path::new("out.pkg")).is_err());
    }
}
