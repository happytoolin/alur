use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use configparser::ini::Ini;

use super::types::PackageManager;

#[derive(Debug, Clone)]
pub struct HniConfig {
    pub default_package_manager: Option<PackageManager>,
    pub global_package_manager: PackageManager,
    pub fast_mode: bool,
    pub config_path: Option<PathBuf>,
}

impl Default for HniConfig {
    fn default() -> Self {
        Self {
            default_package_manager: None,
            global_package_manager: PackageManager::Npm,
            fast_mode: true,
            config_path: None,
        }
    }
}

impl HniConfig {
    pub fn load() -> Result<Self> {
        let mut cfg = Self::default();

        let explicit_path = env::var("HNI_CONFIG_FILE").ok().map(PathBuf::from);

        if let Some(path) = explicit_path {
            let loaded = parse_hnirc_file(&path, &mut cfg, true)
                .map_err(|error| anyhow!("failed to load {}: {error}", path.display()))?;
            if loaded {
                cfg.config_path = Some(path);
            }
        } else if let Some(path) = default_config_path() {
            let loaded = parse_hnirc_file(&path, &mut cfg, false)
                .map_err(|error| anyhow!("failed to load {}: {error}", path.display()))?;
            if loaded {
                cfg.config_path = Some(path);
            }
        }

        if let Ok(v) = env::var("HNI_DEFAULT_PACKAGE_MANAGER") {
            cfg.default_package_manager = Some(parse_pm(&v)?);
        }

        if let Ok(v) = env::var("HNI_GLOBAL_PACKAGE_MANAGER") {
            cfg.global_package_manager = parse_pm(&v)?;
        }

        if let Ok(v) = env::var("HNI_FAST") {
            cfg.fast_mode = parse_bool(&v)?;
        }

        Ok(cfg)
    }
}

fn default_config_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let hni = home.join(".hnirc");
    hni.exists().then_some(hni)
}

fn parse_hnirc_file(path: &Path, config: &mut HniConfig, required: bool) -> Result<bool> {
    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound && !required => return Ok(false),
        Err(error) if error.kind() == io::ErrorKind::NotFound && required => {
            return Err(anyhow!(
                "config error: config file not found: {}",
                path.display()
            ));
        }
        Err(error) => {
            return Err(anyhow!(
                "config error: failed to read config file {}: {error}",
                path.display()
            ));
        }
    };

    let mut ini = Ini::new();
    ini.read(raw)
        .map_err(|error| anyhow!("config error: failed to parse {}: {error}", path.display()))?;

    if let Some(v) = ini.get("default", "defaultpackagemanager") {
        config.default_package_manager = Some(parse_pm(v.trim())?);
    }
    if let Some(v) = ini.get("default", "globalpackagemanager") {
        config.global_package_manager = parse_pm(v.trim())?;
    }
    if let Some(v) = ini.get("default", "fastmode") {
        config.fast_mode = parse_bool(v.trim())?;
    }

    Ok(true)
}

fn parse_pm(value: &str) -> Result<PackageManager> {
    PackageManager::from_name(&value.to_ascii_lowercase())
        .ok_or_else(|| anyhow!("config error: unsupported package manager: {value}"))
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(anyhow!("config error: invalid boolean: {value}")),
    }
}

#[cfg(test)]
fn default_config_path_with_home(home: &Path) -> Option<PathBuf> {
    let hni = home.join(".hnirc");
    hni.exists().then_some(hni)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn parses_hnirc_values() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".hnirc");
        fs::write(
            &path,
            "defaultPackageManager=pnpm\nglobalPackageManager=yarn\nfastMode=false\n",
        )
        .unwrap();

        let mut cfg = HniConfig::default();
        parse_hnirc_file(&path, &mut cfg, true).unwrap();

        assert_eq!(cfg.default_package_manager, Some(PackageManager::Pnpm));
        assert_eq!(cfg.global_package_manager, PackageManager::Yarn);
        assert!(!cfg.fast_mode);
    }

    #[test]
    fn explicit_missing_config_is_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing-hnirc");
        let mut cfg = HniConfig::default();
        let err = parse_hnirc_file(&path, &mut cfg, true).unwrap_err();
        assert!(err.to_string().contains("config file not found"));
    }

    #[test]
    fn default_config_path_only_considers_hnirc() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        fs::write(home.join(".nirc"), "defaultPackageManager=pnpm\n").unwrap();
        fs::write(home.join(".hnirc"), "defaultPackageManager=bun\n").unwrap();

        let resolved = default_config_path_with_home(home).unwrap();
        assert_eq!(resolved, home.join(".hnirc"));
    }

    #[test]
    fn default_config_path_ignores_nirc_when_hnirc_missing() {
        let dir = tempdir().unwrap();
        let home = dir.path();
        fs::write(home.join(".nirc"), "defaultPackageManager=pnpm\n").unwrap();

        assert_eq!(default_config_path_with_home(home), None);
    }
}
