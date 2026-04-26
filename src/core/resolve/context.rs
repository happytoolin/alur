use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::{
    config::HniConfig,
    detect::{
        detect, detect_dev_engines_field, detect_install_metadata_in_dir, detect_lockfile_in_dir,
        detect_package_manager_field, fallback_detection,
    },
    package::NearestPackage,
    pkg_json::{PackageJson, package_json_path, read_package_json},
    types::{DetectionResult, PackageManager},
};

#[derive(Debug)]
pub struct ResolveContext {
    cwd: PathBuf,
    pub config: HniConfig,
    verify_package_manager_availability: bool,
}

impl ResolveContext {
    pub fn new(cwd: PathBuf, config: HniConfig) -> Self {
        Self::with_package_manager_checks(cwd, config, true)
    }

    pub fn with_package_manager_checks(
        cwd: PathBuf,
        config: HniConfig,
        verify_package_manager_availability: bool,
    ) -> Self {
        Self {
            cwd,
            config,
            verify_package_manager_availability,
        }
    }

    pub(crate) fn project_state(&self) -> Result<ProjectState> {
        crate::core::profile::measure("project.scan", || {
            ProjectState::scan(&self.cwd, &self.config)
        })
    }

    pub fn detect(&self) -> Result<crate::core::types::DetectionResult> {
        crate::core::profile::measure("detect.total", || detect(&self.cwd, &self.config))
    }

    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    pub(crate) fn should_verify_package_manager_availability(&self) -> bool {
        self.verify_package_manager_availability
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectState {
    ancestors: Vec<AncestorState>,
    nearest_package: Option<NearestPackage>,
    bin_dirs: Vec<PathBuf>,
    detection: DetectionResult,
}

#[derive(Debug, Clone)]
pub(crate) struct AncestorState {
    dir: PathBuf,
    manifest: Option<PackageJson>,
}

impl ProjectState {
    pub(crate) fn scan(cwd: &Path, config: &HniConfig) -> Result<Self> {
        let mut ancestors = Vec::new();
        let mut nearest_package = None;
        let mut bin_dirs = Vec::new();
        let mut has_lock = false;
        let mut resolved_detection = None;

        for dir in cwd.ancestors() {
            let dir = dir.to_path_buf();
            let should_detect = resolved_detection.is_none() || !has_lock;
            let lockfile_pm = should_detect
                .then(|| detect_lockfile_in_dir(&dir))
                .flatten();
            has_lock |= lockfile_pm.is_some();

            let resolved_agent = resolved_detection
                .as_ref()
                .and_then(|detection: &DetectionResult| detection.agent);
            let needs_nearest_package =
                nearest_package.is_none() && resolved_agent != Some(PackageManager::Deno);
            let manifest = if needs_nearest_package || resolved_detection.is_none() {
                read_package_json(&dir)?
            } else {
                None
            };
            let package_json_path = package_json_path(&dir);

            if nearest_package.is_none()
                && let Some(manifest) = manifest.clone()
            {
                nearest_package = Some(NearestPackage {
                    root: dir.clone(),
                    package_json_path: package_json_path.clone(),
                    manifest,
                });
            }

            if should_detect && resolved_detection.is_none() {
                resolved_detection = manifest
                    .as_ref()
                    .and_then(detect_package_manager_field)
                    .or_else(|| {
                        lockfile_pm.map(|pm| DetectionResult {
                            agent: Some(pm),
                            has_lock,
                            version_hint: None,
                            source: crate::core::types::DetectionSource::Lockfile,
                        })
                    })
                    .or_else(|| manifest.as_ref().and_then(detect_dev_engines_field))
                    .or_else(|| {
                        detect_install_metadata_in_dir(&dir).map(|pm| DetectionResult {
                            agent: Some(pm),
                            has_lock,
                            version_hint: None,
                            source: crate::core::types::DetectionSource::InstallMetadata,
                        })
                    });
            }

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

            ancestors.push(AncestorState { dir, manifest });
        }

        let mut detection =
            resolved_detection.unwrap_or_else(|| fallback_detection(config, has_lock));
        detection.has_lock = has_lock;

        Ok(Self {
            ancestors,
            nearest_package,
            bin_dirs,
            detection,
        })
    }

    pub(crate) fn nearest_package(&self) -> Option<NearestPackage> {
        self.nearest_package.clone()
    }

    pub(crate) fn bin_dirs(&self) -> &[PathBuf] {
        &self.bin_dirs
    }

    pub(crate) fn has_yarn_pnp_loader(&self) -> bool {
        crate::core::profile::measure("project.scan_pnp", || {
            self.ancestors.iter().any(|ancestor| {
                ancestor.dir.join(".pnp.cjs").exists() || ancestor.dir.join(".pnp.js").exists()
            })
        })
    }

    pub(crate) fn detection(&self) -> DetectionResult {
        self.detection.clone()
    }

    pub(crate) fn resolve_declared_package_bin(&self, bin_name: &str) -> Result<Option<PathBuf>> {
        for ancestor in &self.ancestors {
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
}
