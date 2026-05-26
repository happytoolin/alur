use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::{
    config::HniConfig,
    pkg_json::{DeclaredPackageManagerSpec, PackageJson, package_json_path, read_package_json},
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

#[derive(Debug, Clone)]
pub(crate) struct NearestPackage {
    pub root: PathBuf,
    pub package_json_path: PathBuf,
    pub manifest: PackageJson,
}

#[derive(Debug, Clone)]
pub(crate) struct ScannedAncestor {
    pub dir: PathBuf,
    pub manifest: Option<PackageJson>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectDiscovery {
    pub ancestors: Vec<ScannedAncestor>,
    pub nearest_package: Option<NearestPackage>,
    pub bin_dirs: Vec<PathBuf>,
    pub detection: DetectionResult,
    pub has_yarn_pnp_loader: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScanMode {
    Full,
    LocalBinsOnly,
}

impl ProjectDiscovery {
    pub(crate) fn scan(cwd: &Path, config: &HniConfig, mode: ScanMode) -> Result<Self> {
        let mut ancestors = Vec::new();
        let mut nearest_package = None;
        let mut bin_dirs = Vec::new();
        let mut has_lock = false;
        let mut resolved_detection = None;
        let mut has_yarn_pnp_loader = false;

        for dir in cwd.ancestors() {
            let dir = dir.to_path_buf();
            let should_detect = resolved_detection.is_none() || !has_lock;
            let lockfile_pm = should_detect
                .then(|| detect_lockfile_in_dir(&dir))
                .flatten();
            has_lock |= lockfile_pm.is_some();
            has_yarn_pnp_loader |= yarn_pnp_loader_exists(&dir);

            let needs_manifest = matches!(mode, ScanMode::Full)
                && (nearest_package.is_none() || (should_detect && resolved_detection.is_none()));
            let manifest = if needs_manifest {
                read_package_json(&dir)?
            } else {
                None
            };

            if matches!(mode, ScanMode::Full)
                && nearest_package.is_none()
                && let Some(manifest) = manifest.clone()
            {
                nearest_package = Some(NearestPackage {
                    root: dir.clone(),
                    package_json_path: package_json_path(&dir),
                    manifest,
                });
            }

            if should_detect && resolved_detection.is_none() {
                resolved_detection = match mode {
                    ScanMode::Full => manifest
                        .as_ref()
                        .and_then(detect_package_manager_field)
                        .or_else(|| detection_from_lockfile(lockfile_pm, has_lock))
                        .or_else(|| manifest.as_ref().and_then(detect_dev_engines_field))
                        .or_else(|| detection_from_install_metadata(&dir, has_lock)),
                    ScanMode::LocalBinsOnly => detection_from_lockfile(lockfile_pm, has_lock)
                        .or_else(|| detection_from_install_metadata(&dir, has_lock)),
                };
            }

            collect_bin_dirs(&dir, &mut bin_dirs);
            ancestors.push(ScannedAncestor { dir, manifest });

            if scan_complete(
                mode,
                has_lock,
                &resolved_detection,
                nearest_package.is_some(),
            ) {
                break;
            }
        }

        let mut detection = resolved_detection
            .unwrap_or_else(|| fallback_detection_for_mode(mode, config, has_lock));
        detection.has_lock = has_lock;

        Ok(Self {
            ancestors,
            nearest_package,
            bin_dirs,
            detection,
            has_yarn_pnp_loader,
        })
    }
}

fn scan_complete(
    mode: ScanMode,
    has_lock: bool,
    resolved_detection: &Option<DetectionResult>,
    has_nearest_package: bool,
) -> bool {
    if !(has_lock && resolved_detection.is_some()) {
        return false;
    }

    match mode {
        ScanMode::Full => {
            has_nearest_package
                || resolved_detection
                    .as_ref()
                    .and_then(|detection| detection.agent)
                    == Some(PackageManager::Deno)
        }
        ScanMode::LocalBinsOnly => true,
    }
}

fn fallback_detection_for_mode(
    mode: ScanMode,
    config: &HniConfig,
    has_lock: bool,
) -> DetectionResult {
    match mode {
        ScanMode::Full => fallback_detection(config, has_lock),
        ScanMode::LocalBinsOnly => config_only_detection(config, has_lock),
    }
}

pub(crate) fn node_modules_bin_dirs(cwd: &Path) -> Vec<PathBuf> {
    crate::core::profile::measure("local_bin.scan_dirs", || {
        let mut bin_dirs = Vec::new();
        for dir in cwd.ancestors() {
            collect_bin_dirs(dir, &mut bin_dirs);
        }
        bin_dirs
    })
}

pub(crate) fn resolve_declared_package_bin(
    ancestors: &[ScannedAncestor],
    bin_name: &str,
) -> Result<Option<PathBuf>> {
    for ancestor in ancestors {
        let manifest = match &ancestor.manifest {
            Some(manifest) => manifest.clone(),
            None => match read_package_json(&ancestor.dir)? {
                Some(manifest) => manifest,
                None => continue,
            },
        };
        let Some(relative) = manifest.bin_command_path(bin_name) else {
            continue;
        };
        let candidate = ancestor.dir.join(relative);
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
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

fn config_only_detection(config: &HniConfig, has_lock: bool) -> DetectionResult {
    if let Some(agent) = config.default_package_manager {
        return DetectionResult {
            agent: Some(agent),
            has_lock,
            version_hint: None,
            source: DetectionSource::Config,
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
    INSTALL_METADATA
        .iter()
        .find_map(|(entry, pm)| dir.join(entry).exists().then_some(*pm))
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
            DeclaredPackageManagerSpec::Single(entry) => {
                parse_declared_package_manager(entry.name.as_deref()?, entry.version.as_deref())
            }
            DeclaredPackageManagerSpec::Multiple(entries) => entries.iter().find_map(|entry| {
                parse_declared_package_manager(entry.name.as_deref()?, entry.version.as_deref())
            }),
        })
        .map(|(pm, version_hint)| DetectionResult {
            agent: Some(pm),
            has_lock: false,
            version_hint,
            source: DetectionSource::DevEnginesField,
        })
}

pub(crate) fn parse_package_manager_field(value: &str) -> Option<(PackageManager, Option<String>)> {
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
    let raw_version = version.map(str::trim).filter(|value| !value.is_empty());
    let normalized_version = raw_version.and_then(normalize_version_hint);

    let mut pm = PackageManager::from_name(&lower)?;
    if pm == PackageManager::Yarn
        && (raw_version.is_some_and(|value| value.eq_ignore_ascii_case("berry"))
            || normalized_version
                .as_deref()
                .and_then(parse_major)
                .is_some_and(|major| major >= 2))
    {
        pm = PackageManager::YarnBerry;
    }

    let version_hint = if pm == PackageManager::YarnBerry
        && raw_version.is_some_and(|value| value.eq_ignore_ascii_case("berry"))
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

fn parse_major(version: &str) -> Option<u64> {
    if version.eq_ignore_ascii_case("berry") {
        return Some(2);
    }

    version.split('.').next()?.parse::<u64>().ok()
}

pub(crate) fn collect_bin_dirs(dir: &Path, bin_dirs: &mut Vec<PathBuf>) {
    for candidate in [
        dir.join("node_modules").join(".bin"),
        dir.join("node_modules")
            .join(".pnpm")
            .join("node_modules")
            .join(".bin"),
    ] {
        if candidate.is_dir() {
            bin_dirs.push(candidate);
        }
    }
}

fn detection_from_lockfile(
    lockfile_pm: Option<PackageManager>,
    has_lock: bool,
) -> Option<DetectionResult> {
    lockfile_pm.map(|pm| DetectionResult {
        agent: Some(pm),
        has_lock,
        version_hint: None,
        source: DetectionSource::Lockfile,
    })
}

fn detection_from_install_metadata(dir: &Path, has_lock: bool) -> Option<DetectionResult> {
    detect_install_metadata_in_dir(dir).map(|pm| DetectionResult {
        agent: Some(pm),
        has_lock,
        version_hint: None,
        source: DetectionSource::InstallMetadata,
    })
}

fn yarn_pnp_loader_exists(dir: &Path) -> bool {
    dir.join(".pnp.cjs").exists() || dir.join(".pnp.js").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_nearest_package_in_ancestors() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let nested = root.join("packages").join("app");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("package.json"), r#"{"name":"root"}"#).unwrap();

        let discovery =
            ProjectDiscovery::scan(&nested, &HniConfig::default(), ScanMode::Full).unwrap();
        let found = discovery.nearest_package.unwrap();
        assert_eq!(found.root, root);
    }

    #[test]
    fn bin_dirs_are_nearest_first() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let nested = root.join("packages").join("app");
        fs::create_dir_all(root.join("node_modules").join(".bin")).unwrap();
        fs::create_dir_all(nested.join("node_modules").join(".bin")).unwrap();

        let bins = node_modules_bin_dirs(&nested);
        assert_eq!(bins[0], nested.join("node_modules").join(".bin"));
        assert_eq!(bins[1], root.join("node_modules").join(".bin"));
    }

    #[test]
    fn resolves_declared_package_bin_from_scanned_ancestors() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("pkg");
        let nested = pkg.join("src");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(pkg.join("bin")).unwrap();
        fs::write(
            pkg.join("package.json"),
            r#"{"name":"tooling","bin":{"hello":"bin/hello.js"}}"#,
        )
        .unwrap();
        fs::write(pkg.join("bin").join("hello.js"), "console.log('hi')").unwrap();

        let discovery =
            ProjectDiscovery::scan(&nested, &HniConfig::default(), ScanMode::Full).unwrap();
        let resolved = resolve_declared_package_bin(&discovery.ancestors, "hello").unwrap();
        assert_eq!(resolved, Some(pkg.join("bin").join("hello.js")));
    }
}
