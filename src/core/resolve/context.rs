use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::{
    config::HniConfig,
    detect::detect,
    package::NearestPackage,
    pkg_json::{PackageJson, package_json_path, read_package_json},
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
        ProjectState::scan(&self.cwd)
    }

    pub fn detect(&self) -> Result<crate::core::types::DetectionResult> {
        detect(&self.cwd, &self.config)
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
    has_yarn_pnp_loader: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AncestorState {
    dir: PathBuf,
    manifest: Option<PackageJson>,
}

impl ProjectState {
    pub(crate) fn scan(cwd: &Path) -> Result<Self> {
        let mut ancestors = Vec::new();
        let mut nearest_package = None;
        let mut bin_dirs = Vec::new();
        let mut has_yarn_pnp_loader = false;

        for dir in cwd.ancestors() {
            let dir = dir.to_path_buf();
            let manifest = if nearest_package.is_none() {
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

            has_yarn_pnp_loader |= dir.join(".pnp.cjs").exists() || dir.join(".pnp.js").exists();

            ancestors.push(AncestorState { dir, manifest });
        }

        Ok(Self {
            ancestors,
            nearest_package,
            bin_dirs,
            has_yarn_pnp_loader,
        })
    }

    pub(crate) fn nearest_package(&self) -> Option<NearestPackage> {
        self.nearest_package.clone()
    }

    pub(crate) fn bin_dirs(&self) -> &[PathBuf] {
        &self.bin_dirs
    }

    pub(crate) fn has_yarn_pnp_loader(&self) -> bool {
        self.has_yarn_pnp_loader
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
