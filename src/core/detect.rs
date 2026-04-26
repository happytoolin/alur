use std::{env, path::Path};

use anyhow::{Result, anyhow};

use super::{
    config::HniConfig,
    pkg_json::{PackageJson, read_package_json},
    types::{DetectionResult, DetectionSource, PackageManager},
};

const LOCKFILES: &[(&str, PackageManager)] = &[
    ("bun.lockb", PackageManager::Bun),
    ("bun.lock", PackageManager::Bun),
    ("pnpm-lock.yaml", PackageManager::Pnpm),
    ("yarn.lock", PackageManager::Yarn),
    ("package-lock.json", PackageManager::Npm),
    ("npm-shrinkwrap.json", PackageManager::Npm),
    ("deno.lock", PackageManager::Deno),
    ("deno.json", PackageManager::Deno),
    ("deno.jsonc", PackageManager::Deno),
];

const INSTALL_METADATA: &[(&str, PackageManager)] = &[
    ("node_modules/.deno", PackageManager::Deno),
    ("node_modules/.pnpm", PackageManager::Pnpm),
    ("node_modules/.yarn-state.yml", PackageManager::YarnBerry),
    ("node_modules/.yarn_integrity", PackageManager::Yarn),
    ("node_modules/.package-lock.json", PackageManager::Npm),
    (".pnp.cjs", PackageManager::YarnBerry),
    (".pnp.js", PackageManager::YarnBerry),
];

pub fn detect(cwd: &Path, config: &HniConfig) -> Result<DetectionResult> {
    let mut has_lock = false;
    let mut resolved = None;

    for dir in cwd.ancestors() {
        let lockfile_pm = detect_lockfile_in_dir(dir);
        has_lock |= lockfile_pm.is_some();

        if resolved.is_none() {
            let manifest = read_package_json(dir)?;
            resolved = manifest
                .as_ref()
                .and_then(detect_package_manager_field)
                .or_else(|| {
                    lockfile_pm.map(|pm| DetectionResult {
                        agent: Some(pm),
                        has_lock,
                        version_hint: None,
                        source: DetectionSource::Lockfile,
                    })
                })
                .or_else(|| manifest.as_ref().and_then(detect_dev_engines_field))
                .or_else(|| {
                    detect_install_metadata_in_dir(dir).map(|pm| DetectionResult {
                        agent: Some(pm),
                        has_lock,
                        version_hint: None,
                        source: DetectionSource::InstallMetadata,
                    })
                });
        }

        if resolved.is_some() && has_lock {
            break;
        }
    }

    if let Some(mut resolved) = resolved {
        resolved.has_lock = has_lock;
        return Ok(resolved);
    }

    Ok(fallback_detection(config, has_lock))
}

pub(crate) fn fallback_detection(config: &HniConfig, has_lock: bool) -> DetectionResult {
    if let Some(agent) = config.default_package_manager {
        return DetectionResult {
            agent: Some(agent),
            has_lock,
            version_hint: None,
            source: DetectionSource::Config,
        };
    }

    if which::which("npm").is_ok() {
        return DetectionResult {
            agent: Some(PackageManager::Npm),
            has_lock,
            version_hint: None,
            source: DetectionSource::Fallback,
        };
    }

    DetectionResult {
        agent: None,
        has_lock,
        version_hint: None,
        source: DetectionSource::None,
    }
}

pub(crate) fn detect_lockfile_in_dir(dir: &Path) -> Option<PackageManager> {
    LOCKFILES
        .iter()
        .find_map(|(lockfile, pm)| dir.join(lockfile).exists().then_some(*pm))
}

pub(crate) fn detect_install_metadata_in_dir(dir: &Path) -> Option<PackageManager> {
    INSTALL_METADATA.iter().find_map(|(entry, pm)| {
        let candidate = dir.join(entry);
        candidate.exists().then_some(*pm)
    })
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

pub(crate) fn parse_package_manager_field(value: &str) -> Option<(PackageManager, Option<String>)> {
    parse_package_manager_spec(value)
}

pub(crate) fn detect_package_manager_field(package_json: &PackageJson) -> Option<DetectionResult> {
    package_json
        .package_manager
        .as_deref()
        .and_then(parse_package_manager_field)
        .map(|(pm, version_hint)| DetectionResult {
            agent: Some(pm),
            has_lock: false,
            version_hint,
            source: DetectionSource::PackageManagerField,
        })
}

pub(crate) fn detect_dev_engines_field(package_json: &PackageJson) -> Option<DetectionResult> {
    package_json
        .dev_engines
        .as_ref()
        .and_then(|engines| engines.package_manager.as_ref())
        .and_then(|declared| match declared {
            crate::core::pkg_json::DeclaredPackageManagerSpec::Single(entry) => {
                parse_declared_package_manager(entry.name.as_deref()?, entry.version.as_deref())
            }
            crate::core::pkg_json::DeclaredPackageManagerSpec::Multiple(entries) => {
                entries.iter().find_map(|entry| {
                    parse_declared_package_manager(entry.name.as_deref()?, entry.version.as_deref())
                })
            }
        })
        .map(|(pm, version_hint)| DetectionResult {
            agent: Some(pm),
            has_lock: false,
            version_hint,
            source: DetectionSource::DevEnginesField,
        })
}

fn parse_package_manager_spec(value: &str) -> Option<(PackageManager, Option<String>)> {
    let sanitized = value.trim().trim_start_matches(['^', '~']);
    if let Some((name, version)) = sanitized.split_once('@') {
        return parse_declared_package_manager(name, Some(version));
    }

    parse_declared_package_manager(sanitized, None)
}

fn parse_declared_package_manager(
    name: &str,
    version: Option<&str>,
) -> Option<(PackageManager, Option<String>)> {
    let lower = name
        .trim()
        .trim_start_matches(['^', '~'])
        .to_ascii_lowercase();
    let raw_version = version.map(str::trim).filter(|version| !version.is_empty());
    let normalized_version = raw_version.and_then(normalize_version_hint);

    let mut pm = PackageManager::from_name(&lower)?;
    if pm == PackageManager::Yarn
        && (raw_version.is_some_and(|version| version.eq_ignore_ascii_case("berry"))
            || normalized_version
                .as_deref()
                .and_then(parse_major)
                .is_some_and(|major| major >= 2))
    {
        pm = PackageManager::YarnBerry;
    }

    let version_hint = if pm == PackageManager::YarnBerry
        && raw_version.is_some_and(|version| version.eq_ignore_ascii_case("berry"))
    {
        None
    } else {
        normalized_version
    };

    Some((pm, version_hint))
}

fn normalize_version_hint(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("berry") {
        return Some("berry".to_string());
    }

    let start = trimmed.find(|char: char| char.is_ascii_digit())?;
    let suffix = &trimmed[start..];
    let len = suffix
        .chars()
        .take_while(|char| char.is_ascii_digit() || *char == '.')
        .map(char::len_utf8)
        .sum::<usize>();

    (len > 0).then(|| suffix[..len].to_string())
}

fn parse_user_agent(value: &str) -> Option<PackageManager> {
    let name = value.split('/').next()?.trim().to_ascii_lowercase();
    match name.as_str() {
        "yarn" => Some(PackageManager::Yarn),
        other => PackageManager::from_name(other),
    }
}

fn parse_major(version: &str) -> Option<u64> {
    if version.eq_ignore_ascii_case("berry") {
        return Some(2);
    }

    version.split('.').next()?.parse::<u64>().ok()
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
