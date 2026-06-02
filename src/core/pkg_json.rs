use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
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
    let raw = match crate::core::profile::measure("package_json.read_file", || fs::read(&path)) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("config error: failed to read {}", path.display()));
        }
    };

    let parsed: PackageJson =
        crate::core::profile::measure("package_json.parse_serde", || serde_json::from_slice(&raw))
            .with_context(|| format!("config error: failed to parse {}", path.display()))?;
    Ok(Some(parsed))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, fs};

    use super::{PackageBin, PackageJson, package_json_path, read_package_json};

    #[test]
    fn package_json_path_points_at_package_manifest() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(
            package_json_path(dir.path()),
            dir.path().join("package.json")
        );
    }

    #[test]
    fn bin_command_path_handles_single_and_mapped_bins() {
        let single = PackageJson {
            name: Some("@scope/tool".to_string()),
            bin: PackageBin::Single("bin/tool.js".to_string()),
            ..PackageJson::default()
        };
        assert_eq!(single.bin_command_path("tool"), Some("bin/tool.js"));
        assert_eq!(single.bin_command_path("@scope/tool"), None);

        let mapped = PackageJson {
            bin: PackageBin::Map(BTreeMap::from([(
                "hni".to_string(),
                "dist/index.js".to_string(),
            )])),
            ..PackageJson::default()
        };
        assert_eq!(mapped.bin_command_path("hni"), Some("dist/index.js"));
        assert_eq!(mapped.bin_command_path("missing"), None);
    }

    #[test]
    fn read_package_json_distinguishes_missing_parse_and_success() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_package_json(dir.path()).unwrap().is_none());

        fs::write(dir.path().join("package.json"), "{ invalid").unwrap();
        let error = read_package_json(dir.path()).unwrap_err().to_string();
        assert!(error.contains("failed to parse"));

        fs::write(
            dir.path().join("package.json"),
            r#"{"name":"demo","bin":{"demo":"bin/demo.js"}}"#,
        )
        .unwrap();
        let parsed = read_package_json(dir.path()).unwrap().unwrap();
        assert_eq!(parsed.bin_command_path("demo"), Some("bin/demo.js"));
    }

    #[test]
    fn read_package_json_reports_read_errors() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("package.json")).unwrap();

        let error = read_package_json(dir.path()).unwrap_err().to_string();
        assert!(error.contains("failed to read"));
    }
}
