use std::{env, path::Path};

use anyhow::{Result, anyhow};

use super::{
    config::HniConfig,
    project::{ProjectDiscovery, ScanMode},
    types::{DetectionResult, PackageManager},
};
#[cfg(test)]
use super::{
    project::{
        detect_dev_engines_field, detect_install_metadata_in_dir, detect_lockfile_in_dir,
        detect_package_manager_field, parse_package_manager_field,
    },
    types::DetectionSource,
};

pub fn detect(cwd: &Path, config: &HniConfig) -> Result<DetectionResult> {
    Ok(ProjectDiscovery::scan(cwd, config, ScanMode::Full)?.detection)
}

pub fn detect_user_agent() -> Option<PackageManager> {
    let user_agent = env::var("npm_config_user_agent").ok()?;
    parse_user_agent(&user_agent)
}

pub fn ensure_package_manager_available(
    pm: PackageManager,
    version_hint: Option<&str>,
) -> Result<()> {
    if env::var_os("HNI_SKIP_PM_CHECK").is_some() {
        return Ok(());
    }

    if which::which(pm.bin()).is_ok() {
        return Ok(());
    }

    let package = pm.global_package_name();
    let target = match version_hint {
        Some(version) if !version.is_empty() => format!("{package}@{version}"),
        _ => package.to_string(),
    };

    if package == "npm" {
        return Err(anyhow!(
            "detected {} but it is not installed.\nInstall Node.js/npm first: https://nodejs.org/",
            pm.display_name(),
        ));
    }

    if matches!(pm, PackageManager::Deno) {
        return Err(anyhow!(
            "detected {} but it is not installed.\nInstall Deno manually: https://deno.com/",
            pm.display_name(),
        ));
    }

    Err(anyhow!(
        "detected {} but it is not installed.\nTry: npm i -g {target}",
        pm.display_name(),
    ))
}

fn parse_user_agent(value: &str) -> Option<PackageManager> {
    let name = value.split('/').next()?.trim().to_ascii_lowercase();
    match name.as_str() {
        "yarn" => Some(PackageManager::Yarn),
        other => PackageManager::from_name(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{config::HniConfig, types::DetectionSource};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn detects_package_manager_field_first() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"packageManager":"pnpm@9.0.0"}"#,
        )
        .unwrap();

        let out = detect(dir.path(), &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Pnpm));
        assert_eq!(out.source, DetectionSource::PackageManagerField);
    }

    #[test]
    fn detects_lockfile_priority() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("yarn.lock"), "x").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "x").unwrap();

        let out = detect(dir.path(), &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Pnpm));
    }

    #[test]
    fn lockfile_priority_prefers_bun_when_multiple_lockfiles_exist() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package-lock.json"), "x").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "x").unwrap();
        fs::write(dir.path().join("bun.lockb"), "x").unwrap();

        let out = detect(dir.path(), &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Bun));
    }

    #[test]
    fn package_manager_field_yarn_berry() {
        let parsed = parse_package_manager_field("yarn@4.2.1").unwrap();
        assert_eq!(parsed.0, PackageManager::YarnBerry);
    }

    #[test]
    fn package_manager_field_name_is_case_insensitive() {
        let parsed = parse_package_manager_field("PNPM@9.0.0").unwrap();
        assert_eq!(parsed.0, PackageManager::Pnpm);
        assert_eq!(parsed.1.as_deref(), Some("9.0.0"));
    }

    #[test]
    fn package_manager_field_short_major_yarn_is_berry() {
        let parsed = parse_package_manager_field("yarn@4").unwrap();
        assert_eq!(parsed.0, PackageManager::YarnBerry);
        assert_eq!(parsed.1.as_deref(), Some("4"));
    }

    #[test]
    fn package_manager_field_short_minor_yarn_is_berry() {
        let parsed = parse_package_manager_field("yarn@4.2").unwrap();
        assert_eq!(parsed.0, PackageManager::YarnBerry);
        assert_eq!(parsed.1.as_deref(), Some("4.2"));
    }

    #[test]
    fn package_manager_field_without_version_is_supported() {
        let parsed = parse_package_manager_field("pnpm").unwrap();
        assert_eq!(parsed.0, PackageManager::Pnpm);
        assert_eq!(parsed.1, None);
    }

    #[test]
    fn package_manager_field_unknown_manager_is_ignored() {
        assert!(parse_package_manager_field("foo@1.0.0").is_none());
    }

    #[test]
    fn package_manager_field_range_normalizes_version() {
        let parsed = parse_package_manager_field("^pnpm@8.1.0").unwrap();
        assert_eq!(parsed.0, PackageManager::Pnpm);
        assert_eq!(parsed.1.as_deref(), Some("8.1.0"));
    }

    #[test]
    fn pnpm_workspace_manifest_is_not_a_lockfile() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - packages/*\n",
        )
        .unwrap();

        assert_eq!(detect_lockfile_in_dir(dir.path()), None);
    }

    #[test]
    fn install_metadata_does_not_treat_bun_lockfiles_as_metadata() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("bun.lockb"), "").unwrap();

        assert_eq!(detect_install_metadata_in_dir(dir.path()), None);
    }

    #[test]
    fn detect_dev_engines_field_supports_array_form() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{
              "devEngines": {
                "packageManager": [
                  { "name": "pnpm", "version": "9.0.0" }
                ]
              }
            }"#,
        )
        .unwrap();

        let out = detect(dir.path(), &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Pnpm));
        assert_eq!(out.source, DetectionSource::DevEnginesField);
        assert_eq!(out.version_hint.as_deref(), Some("9.0.0"));
    }

    #[test]
    fn user_agent_detection_is_coarse() {
        assert_eq!(
            parse_user_agent("yarn/4.2.0 npm/? node/v20.0.0 darwin x64"),
            Some(PackageManager::Yarn)
        );
    }
}
