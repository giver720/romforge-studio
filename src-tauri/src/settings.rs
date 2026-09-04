use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Ruta explicita a chdman elegida por el usuario. Si esta vacia se autodetecta.
    pub chdman_path: Option<String>,
    /// Carpeta de salida fija. Si es None se usa la carpeta del archivo de entrada.
    pub output_dir: Option<String>,
    /// Preset de compresion por defecto: "max" | "balanced" | "fast"
    pub preset: String,
    /// Borrar el archivo de origen cuando el trabajo termina bien.
    pub delete_source: bool,
    /// Sobrescribir archivos de salida existentes (-f).
    pub overwrite: bool,
    /// Trabajos simultaneos (chdman ya usa varios hilos internamente).
    pub parallel: usize,
    /// Hilos por trabajo (-np). 0 = automatico.
    pub threads: u32,
    /// Color de acento de la interfaz.
    pub accent: String,
    /// Consultar la biblioteca publica de Libretro cuando no hay portada local.
    pub online_artwork: bool,
    /// Rutas elegidas a mano para herramientas externas, por id.
    pub tool_paths: std::collections::HashMap<String, String>,
    /// Ruta elegida a mano para las prod.keys de Switch.
    pub switch_keys_path: Option<String>,
    /// Ruta elegida a mano para boot9.bin (3DS).
    pub boot9_path: Option<String>,
    /// Ruta elegida a mano para aes_keys.txt (3DS).
    pub aes_keys_path: Option<String>,
    /// Ruta a seeddb.bin, que hace falta para los juegos con cifrado por semilla.
    pub seeddb_path: Option<String>,
    /// Nivel de compresion para nsz (por defecto 18; el maximo util es 22).
    pub nsz_level: u8,
    /// Reservar mas hilos a nsz; 0 = automatico.
    pub nsz_threads: u32,
    /// Recortar el espacio vacio del ISO al pasarlo a GOD.
    pub xbox_trim: bool,
    /// Saltarse $SystemUpdate al extraer un ISO de Xbox a carpeta.
    pub xbox_skip_update: bool,
    /// Partir los archivos grandes de PS3 en trozos de 4 GB para FAT32.
    pub ps3_split_fat32: bool,
    /// Quitar los datos de relleno del disco al convertir juegos de Wii.
    pub wii_scrub: bool,
    /// Nivel de compresion zstd para RVZ (Dolphin recomienda 5).
    pub wii_level: u8,
    /// Partir el WBFS en trozos para que quepa en FAT32.
    pub wii_wbfs_split: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            chdman_path: None,
            output_dir: None,
            preset: "balanced".into(),
            delete_source: false,
            overwrite: false,
            parallel: 1,
            threads: 0,
            accent: "violet".into(),
            online_artwork: true,
            tool_paths: Default::default(),
            switch_keys_path: None,
            boot9_path: None,
            aes_keys_path: None,
            seeddb_path: None,
            nsz_level: 18,
            nsz_threads: 0,
            xbox_trim: true,
            xbox_skip_update: true,
            ps3_split_fat32: false,
            wii_scrub: true,
            wii_level: 5,
            wii_wbfs_split: true,
        }
    }
}

/// Carpeta junto al ejecutable, si la version portable la marca con un archivo.
///
/// La compilacion portable incluye un `portable.txt` al lado del .exe; cuando
/// existe, todo (ajustes, herramientas descargadas, entorno de Python) se
/// guarda ahi en vez de en %APPDATA%, para poder llevarlo en un USB.
fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    if dir.join("portable.txt").is_file() {
        Some(dir.join("datos"))
    } else {
        None
    }
}

pub fn is_portable() -> bool {
    portable_dir().is_some()
}

pub fn config_dir() -> PathBuf {
    if let Some(p) = portable_dir() {
        return p;
    }
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    migrated_config_dir(&base)
}

/// Conserva los ajustes, herramientas y portadas de instalaciones anteriores.
/// El cambio de nombre se hace dentro del mismo disco, por lo que `rename` es
/// atomico. Si el sistema no permite mover la carpeta, seguimos usando la ruta
/// antigua para no dejar al usuario sin sus datos.
fn migrated_config_dir(base: &std::path::Path) -> PathBuf {
    let current = base.join("romforge-studio");
    if current.exists() {
        return current;
    }

    let legacy = base.join("chd-studio");
    if legacy.is_dir() && std::fs::rename(&legacy, &current).is_err() {
        return legacy;
    }

    current
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

/// Carpeta donde la app guarda su copia privada de chdman.
pub fn bin_dir() -> PathBuf {
    config_dir().join("bin")
}

pub fn load() -> Settings {
    match std::fs::read_to_string(settings_file()) {
        Ok(txt) => serde_json::from_str(&txt).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(s: &Settings) -> anyhow::Result<()> {
    std::fs::create_dir_all(config_dir())?;
    std::fs::write(settings_file(), serde_json::to_string_pretty(s)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::migrated_config_dir;

    #[test]
    fn migrates_the_legacy_configuration_directory() {
        let base = std::env::temp_dir().join(format!(
            "romforge-settings-migration-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        let legacy = base.join("chd-studio");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(legacy.join("settings.json"), "{}").unwrap();

        let migrated = migrated_config_dir(&base);

        assert_eq!(migrated, base.join("romforge-studio"));
        assert!(migrated.join("settings.json").is_file());
        assert!(!legacy.exists());
        let _ = std::fs::remove_dir_all(&base);
    }
}
