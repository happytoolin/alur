use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use semver::Version;

use super::{
    config::{DefaultAgent, HniConfig},
    error::{HniError, HniResult},
    pkg_json::PackageJson,
    resolve::ProjectState,
    types::{DetectionResult, DetectionSource, PackageManager},
};

const LOCKFILES: &[(&str, PackageManager)] = &[
    ("bun.lockb", PackageManager::Bun),
    ("bun.lock", PackageManager::Bun),
    ("pnpm-lock.yaml", PackageManager::Pnpm),
    ("pnpm-workspace.yaml", PackageManager::Pnpm),
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
    ("bun.lock", PackageManager::Bun),
    ("bun.lockb", PackageManager::Bun),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectStrategy {
    PackageManagerField,
    Lockfile,
    DevEnginesField,
    InstallMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectOptions {
    pub strategies: Vec<DetectStrategy>,
    pub stop_at: Option<PathBuf>,
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self {
            strategies: vec![
                DetectStrategy::PackageManagerField,
                DetectStrategy::Lockfile,
                DetectStrategy::DevEnginesField,
                DetectStrategy::InstallMetadata,
            ],
            stop_at: None,
        }
    }
}

pub fn detect(cwd: &Path, config: &HniConfig) -> HniResult<DetectionResult> {
    detect_with_options(cwd, config, &DetectOptions::default())
}

pub fn detect_with_options(
    cwd: &Path,
    config: &HniConfig,
    options: &DetectOptions,
) -> HniResult<DetectionResult> {
    Ok(detect_in_project_state(
        &ProjectState::scan(cwd)?,
        cwd,
        config,
        options,
    ))
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

pub(crate) fn detect_in_project_state(
    state: &ProjectState,
    cwd: &Path,
    config: &HniConfig,
    options: &DetectOptions,
) -> DetectionResult {
    let stop_at = options.stop_at.as_ref().map(|path| {
        if path.is_absolute() {
            path.clone()
        } else {
            cwd.join(path)
        }
    });

    let mut has_lock = false;
    let mut resolved = None;

    for ancestor in state.ancestors() {
        has_lock |= ancestor.lockfile_pm().is_some();

        if resolved.is_none() {
            for strategy in &options.strategies {
                let candidate = match strategy {
                    DetectStrategy::PackageManagerField => {
                        ancestor.manifest().and_then(detect_package_manager_field)
                    }
                    DetectStrategy::Lockfile => ancestor.lockfile_pm().map(|pm| DetectionResult {
                        agent: Some(pm),
                        has_lock,
                        version_hint: None,
                        source: DetectionSource::Lockfile,
                    }),
                    DetectStrategy::DevEnginesField => {
                        ancestor.manifest().and_then(detect_dev_engines_field)
                    }
                    DetectStrategy::InstallMetadata => {
                        detect_install_metadata_in_dir(ancestor.dir()).map(|pm| DetectionResult {
                            agent: Some(pm),
                            has_lock,
                            version_hint: None,
                            source: DetectionSource::InstallMetadata,
                        })
                    }
                };

                if let Some(candidate) = candidate {
                    resolved = Some(candidate);
                    break;
                }
            }
        }

        if stop_at.as_ref().is_some_and(|stop| ancestor.dir() == stop) {
            break;
        }

        if resolved.is_some() && has_lock {
            break;
        }
    }

    if let Some(mut resolved) = resolved {
        resolved.has_lock = has_lock;
        return resolved;
    }

    if let DefaultAgent::Agent(agent) = config.default_agent {
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

pub fn ensure_package_manager_available(
    pm: PackageManager,
    version_hint: Option<&str>,
    config: &HniConfig,
    cwd: &Path,
) -> HniResult<()> {
    if env::var_os("HNI_SKIP_PM_CHECK").is_some() {
        return Ok(());
    }

    if which::which(pm.bin()).is_ok() {
        return Ok(());
    }

    if !config.auto_install {
        let install_hint = format!("npm i -g {}", pm.global_package_name());
        return Err(HniError::detection(format!(
            "detected {} but it is not installed.\nTry: {install_hint}\nOr set HNI_AUTO_INSTALL=true",
            pm.display_name(),
        )));
    }

    if env::var_os("CI").is_some() {
        eprintln!("[hni] auto-installing {} in CI mode", pm.display_name());
    }

    let package = pm.global_package_name();
    if package == "npm" {
        return Err(HniError::detection(
            "npm is required for auto-install but was not found in PATH",
        ));
    }

    if matches!(pm, PackageManager::Deno) {
        return Err(HniError::detection(
            "auto-install for deno is not supported; install deno manually",
        ));
    }

    if which::which("npm").is_err() {
        return Err(HniError::detection(
            "auto-install requires npm in PATH, but npm is unavailable.\nInstall Node.js/npm first: https://nodejs.org/",
        ));
    }

    let target = match version_hint {
        Some(version) if !version.is_empty() => format!("{package}@{version}"),
        _ => package.to_string(),
    };

    let status = Command::new("npm")
        .args(["i", "-g", &target])
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| {
            HniError::detection(format!("failed to run npm for auto-install: {error}"))
        })?;

    if !status.success() {
        return Err(HniError::detection(format!(
            "auto-install failed for {} with exit code {:?}",
            pm.display_name(),
            status.code()
        )));
    }

    if which::which(pm.bin()).is_err() {
        return Err(HniError::detection(format!(
            "auto-install for {} completed but binary is still not in PATH",
            pm.display_name()
        )));
    }

    Ok(())
}

pub(crate) fn parse_package_manager_field(value: &str) -> Option<(PackageManager, Option<String>)> {
    parse_package_manager_spec(value)
}

fn detect_package_manager_field(package_json: &PackageJson) -> Option<DetectionResult> {
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

fn detect_dev_engines_field(package_json: &PackageJson) -> Option<DetectionResult> {
    package_json
        .dev_engines
        .as_ref()
        .and_then(|engines| engines.package_manager.as_ref())
        .and_then(|declared| {
            parse_declared_package_manager(declared.name.as_deref()?, declared.version.as_deref())
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

    Version::parse(version)
        .map(|parsed| parsed.major)
        .ok()
        .or_else(|| {
            Version::parse(&format!("{version}.0.0"))
                .map(|parsed| parsed.major)
                .ok()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::HniConfig;
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
    fn detect_with_options_respects_stop_at() {
        let root = tempdir().unwrap();
        let stop_at = root.path().join("no-files");
        let nested = stop_at.join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.path().join("package-lock.json"), "lock").unwrap();

        let detected = detect_with_options(
            &nested,
            &HniConfig::default(),
            &DetectOptions {
                stop_at: Some(stop_at.clone()),
                ..DetectOptions::default()
            },
        )
        .unwrap();

        assert_ne!(detected.source, DetectionSource::Lockfile);
    }

    #[test]
    fn user_agent_detection_is_coarse() {
        assert_eq!(
            parse_user_agent("yarn/4.2.0 npm/? node/v20.0.0 darwin x64"),
            Some(PackageManager::Yarn)
        );
    }

    #[test]
    fn yarn_lock_without_package_manager_stays_yarn_classic() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("yarn.lock"), "lock").unwrap();
        fs::write(dir.path().join(".yarnrc.yml"), "nodeLinker: pnp\n").unwrap();

        let out = detect(dir.path(), &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Yarn));
    }

    #[test]
    fn detects_workspace_root_package_manager_from_subpackage() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("package.json"),
            r#"{"packageManager":"pnpm@9.0.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        fs::write(root.path().join("pnpm-lock.yaml"), "lock").unwrap();

        let pkg = root.path().join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("package.json"), r#"{"name":"app"}"#).unwrap();

        let out = detect(&pkg, &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Pnpm));
        assert_eq!(out.source, DetectionSource::PackageManagerField);
        assert!(out.has_lock);
    }

    #[test]
    fn detects_workspace_lockfile_from_subpackage() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("pnpm-lock.yaml"), "lock").unwrap();

        let pkg = root.path().join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("package.json"), r#"{"name":"app"}"#).unwrap();

        let out = detect(&pkg, &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Pnpm));
        assert_eq!(out.source, DetectionSource::Lockfile);
        assert!(out.has_lock);
    }

    #[test]
    fn prefers_subpackage_lockfile_over_parent_package_manager() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("package.json"),
            r#"{"packageManager":"pnpm@9.0.0"}"#,
        )
        .unwrap();

        let pkg = root.path().join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(pkg.join("package.json"), r#"{"name":"app"}"#).unwrap();
        fs::write(pkg.join("package-lock.json"), "lock").unwrap();

        let out = detect(&pkg, &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Npm));
        assert_eq!(out.source, DetectionSource::Lockfile);
        assert!(out.has_lock);
    }

    #[test]
    fn has_lock_tracks_ancestor_lock_even_when_agent_is_from_subpackage_package_manager_field() {
        let root = tempdir().unwrap();
        fs::write(root.path().join("pnpm-lock.yaml"), "lock").unwrap();

        let pkg = root.path().join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            pkg.join("package.json"),
            r#"{"name":"app","packageManager":"npm@10.0.0"}"#,
        )
        .unwrap();

        let out = detect(&pkg, &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Npm));
        assert_eq!(out.source, DetectionSource::PackageManagerField);
        assert!(out.has_lock);
    }

    #[test]
    fn detects_deno_from_deno_json() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("deno.json"),
            r#"{"tasks":{"dev":"deno test"}}"#,
        )
        .unwrap();

        let out = detect(dir.path(), &HniConfig::default()).unwrap();
        assert_eq!(out.agent, Some(PackageManager::Deno));
        assert_eq!(out.source, DetectionSource::Lockfile);
        assert!(out.has_lock);
    }
}
