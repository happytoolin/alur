use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::core::{
    config::HniConfig,
    detect::{
        DetectOptions, detect_in_project_state, detect_install_metadata_in_dir,
        detect_lockfile_in_dir,
    },
    error::HniResult,
    package::NearestPackage,
    pkg_json::{PackageJson, package_json_path, read_package_json},
    types::{DetectionResult, PackageManager},
};

#[derive(Debug)]
pub struct ResolveContext {
    cwd: PathBuf,
    pub config: HniConfig,
    verify_package_manager_availability: bool,
    project_state: OnceLock<ProjectState>,
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
            project_state: OnceLock::new(),
        }
    }

    pub(crate) fn project_state(&self) -> HniResult<&ProjectState> {
        if let Some(state) = self.project_state.get() {
            return Ok(state);
        }

        let state = ProjectState::scan(&self.cwd)?;
        let _ = self.project_state.set(state);
        Ok(self
            .project_state
            .get()
            .expect("project state should be initialized"))
    }

    pub fn detect(&self) -> HniResult<DetectionResult> {
        Ok(self.project_state()?.detect(&self.config))
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
    lockfile_pm: Option<PackageManager>,
    install_metadata_pm: Option<PackageManager>,
}

impl ProjectState {
    pub(crate) fn scan(cwd: &Path) -> HniResult<Self> {
        let mut ancestors = Vec::new();
        let mut nearest_package = None;
        let mut bin_dirs = Vec::new();
        let mut has_yarn_pnp_loader = false;

        for dir in cwd.ancestors() {
            let dir = dir.to_path_buf();
            let manifest = read_package_json(&dir)?;
            let package_json_path = package_json_path(&dir);
            let lockfile_pm = detect_lockfile_in_dir(&dir);
            let install_metadata_pm = detect_install_metadata_in_dir(&dir);

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

            ancestors.push(AncestorState {
                dir,
                manifest,
                lockfile_pm,
                install_metadata_pm,
            });
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

    pub(crate) fn resolve_declared_package_bin(&self, bin_name: &str) -> Option<PathBuf> {
        self.ancestors.iter().find_map(|ancestor| {
            let manifest = ancestor.manifest.as_ref()?;
            let relative = manifest.bin_command_path(bin_name)?;
            let candidate = ancestor.dir.join(relative);
            candidate.is_file().then_some(candidate)
        })
    }

    pub(crate) fn ancestors(&self) -> &[AncestorState] {
        &self.ancestors
    }

    pub(crate) fn detect(&self, config: &HniConfig) -> DetectionResult {
        detect_in_project_state(
            self,
            self.ancestors
                .first()
                .map(|ancestor| ancestor.dir.as_path())
                .unwrap_or_else(|| Path::new(".")),
            config,
            &DetectOptions::default(),
        )
    }
}

impl AncestorState {
    pub(crate) fn dir(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn manifest(&self) -> Option<&PackageJson> {
        self.manifest.as_ref()
    }

    pub(crate) fn lockfile_pm(&self) -> Option<PackageManager> {
        self.lockfile_pm
    }

    pub(crate) fn install_metadata_pm(&self) -> Option<PackageManager> {
        self.install_metadata_pm
    }
}
