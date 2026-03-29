use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PackageJson {
    pub name: Option<String>,
    #[serde(rename = "packageManager")]
    pub package_manager: Option<String>,
    #[serde(rename = "devEngines")]
    pub dev_engines: Option<DevEngines>,
    #[serde(default)]
    pub bin: PackageBin,
    pub scripts: Option<BTreeMap<String, String>>,
    #[serde(rename = "scripts-info")]
    pub scripts_info: Option<BTreeMap<String, String>>,
    pub dependencies: Option<BTreeMap<String, String>>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<BTreeMap<String, String>>,
    #[serde(rename = "peerDependencies")]
    pub peer_dependencies: Option<BTreeMap<String, String>>,
    #[serde(rename = "optionalDependencies")]
    pub optional_dependencies: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DevEngines {
    #[serde(rename = "packageManager")]
    pub package_manager: Option<DeclaredPackageManagerSpec>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DeclaredPackageManager {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DeclaredPackageManagerSpec {
    Single(DeclaredPackageManager),
    Multiple(Vec<DeclaredPackageManager>),
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum PackageBin {
    #[default]
    None,
    Single(String),
    Map(BTreeMap<String, String>),
}

impl PackageJson {
    pub fn bin_command_path(&self, command: &str) -> Option<&str> {
        match &self.bin {
            PackageBin::None => None,
            PackageBin::Single(path) => {
                let package_name = self.name.as_deref()?;
                let short_name = package_name
                    .rsplit_once('/')
                    .map(|(_, tail)| tail)
                    .unwrap_or(package_name);
                if short_name == command {
                    Some(path.as_str())
                } else {
                    None
                }
            }
            PackageBin::Map(map) => map.get(command).map(String::as_str),
        }
    }
}

pub fn package_json_path(cwd: &Path) -> PathBuf {
    cwd.join("package.json")
}

pub fn read_package_json(cwd: &Path) -> Result<Option<PackageJson>> {
    let path = package_json_path(cwd);
    let raw = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(anyhow!(
                "config error: failed to read {}: {e}",
                path.display()
            ));
        }
    };

    let parsed: PackageJson = serde_json::from_str(&raw)
        .map_err(|error| anyhow!("config error: failed to parse {}: {error}", path.display()))?;
    Ok(Some(parsed))
}
