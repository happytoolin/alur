use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::Deserialize;
use thiserror::Error;

use super::types::PackageManager;

#[derive(Debug, Clone)]
pub struct AlurConfig {
    pub default_package_manager: Option<PackageManager>,
    pub global_package_manager: PackageManager,
    pub fast_mode: bool,
    pub config_path: Option<PathBuf>,
}

impl Default for AlurConfig {
    fn default() -> Self {
        Self {
            default_package_manager: None,
            global_package_manager: PackageManager::Npm,
            fast_mode: true,
            config_path: None,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct AlurConfigValues {
    default_package_manager: Option<PackageManager>,
    global_package_manager: Option<PackageManager>,
    fast_mode: Option<bool>,
}

#[derive(Debug, Error)]
enum ConfigError {
    #[error("config error: config file not found: {0}")]
    FileNotFound(PathBuf),
}

impl AlurConfig {
    pub fn load() -> Result<Self> {
        let explicit_path = env::var_os("ALUR_CONFIG_FILE").map(PathBuf::from);
        let config_path = match explicit_path {
            Some(path) if path.exists() => Some(path),
            Some(path) => {
                let display = path.display().to_string();
                return Err(anyhow::Error::new(ConfigError::FileNotFound(path)))
                    .with_context(|| format!("failed to load {display}"));
            }
            None => default_config_path().filter(|path| path.exists()),
        };

        let mut figment = Figment::new();
        if let Some(path) = &config_path {
            figment = figment.merge(Toml::file(path));
        }

        let values: AlurConfigValues = figment
            .merge(Env::prefixed("ALUR_"))
            .extract()
            .context("config error: failed to load configuration")?;

        let default = Self::default();
        Ok(Self {
            default_package_manager: values.default_package_manager,
            global_package_manager: values
                .global_package_manager
                .unwrap_or(default.global_package_manager),
            fast_mode: values.fast_mode.unwrap_or(default.fast_mode),
            config_path,
        })
    }
}

fn default_config_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("alur").join("config.toml"))
}

#[cfg(test)]
fn default_config_path_with_config_dir(config_dir: &std::path::Path) -> PathBuf {
    config_dir.join("alur").join("config.toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use figment::{
        Figment,
        providers::{Format, Toml},
    };
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_toml_values() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "default_package_manager = \"pnpm\"\nglobal_package_manager = \"yarn\"\nfast_mode = false\n",
        )
        .unwrap();

        let values: AlurConfigValues = Figment::new().merge(Toml::file(&path)).extract().unwrap();

        assert_eq!(values.default_package_manager, Some(PackageManager::Pnpm));
        assert_eq!(values.global_package_manager, Some(PackageManager::Yarn));
        assert_eq!(values.fast_mode, Some(false));
    }

    #[test]
    fn explicit_missing_config_is_error() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing-config.toml");
        let err = ConfigError::FileNotFound(path).to_string();
        assert!(err.to_string().contains("config file not found"));
    }

    #[test]
    fn default_config_path_uses_config_toml() {
        let dir = tempdir().unwrap();
        let resolved = default_config_path_with_config_dir(dir.path());
        assert_eq!(resolved, dir.path().join("alur").join("config.toml"));
    }
}
