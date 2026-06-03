use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::core::{
    config::HniConfig,
    detect::detect,
    project::{
        NearestPackage, ProjectDiscovery, ScanMode, ScannedAncestor, resolve_declared_package_bin,
    },
    types::DetectionResult,
};

#[derive(Debug)]
pub struct ResolveContext {
    cwd: PathBuf,
    pub config: HniConfig,
    verify_package_manager_availability: bool,
}

impl ResolveContext {
    #[must_use]
    pub fn new(cwd: PathBuf, config: HniConfig) -> Self {
        Self::with_package_manager_checks(cwd, config, true)
    }

    #[must_use]
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

    pub(crate) fn local_bin_project_state(&self) -> Result<LocalBinProjectState> {
        crate::core::profile::measure("local_bin.scan_project", || {
            LocalBinProjectState::scan(&self.cwd, &self.config)
        })
    }

    /// Detects the package-manager context for the current working directory.
    ///
    /// # Errors
    ///
    /// Returns an error when project discovery or package-manager detection fails.
    pub fn detect(&self) -> Result<crate::core::types::DetectionResult> {
        crate::core::profile::measure("detect.total", || detect(&self.cwd, &self.config))
    }

    #[must_use]
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    #[must_use]
    pub(crate) fn should_verify_package_manager_availability(&self) -> bool {
        self.verify_package_manager_availability
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectState {
    nearest_package: Option<NearestPackage>,
    bin_dirs: Vec<PathBuf>,
    detection: DetectionResult,
    has_yarn_pnp_loader: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct LocalBinProjectState {
    ancestors: Vec<ScannedAncestor>,
    bin_dirs: Vec<PathBuf>,
    detection: DetectionResult,
    has_yarn_pnp_loader: bool,
}

impl ProjectState {
    pub(crate) fn scan(cwd: &Path, config: &HniConfig) -> Result<Self> {
        let discovery = ProjectDiscovery::scan(cwd, config, ScanMode::Full)?;

        Ok(Self {
            nearest_package: discovery.nearest_package,
            bin_dirs: discovery.bin_dirs,
            detection: discovery.detection,
            has_yarn_pnp_loader: discovery.has_yarn_pnp_loader,
        })
    }

    #[must_use]
    pub(crate) fn nearest_package(&self) -> Option<NearestPackage> {
        self.nearest_package.clone()
    }

    #[must_use]
    pub(crate) fn bin_dirs(&self) -> &[PathBuf] {
        &self.bin_dirs
    }

    #[must_use]
    pub(crate) fn has_yarn_pnp_loader(&self) -> bool {
        self.has_yarn_pnp_loader
    }

    #[must_use]
    pub(crate) fn detection(&self) -> DetectionResult {
        self.detection.clone()
    }
}

impl LocalBinProjectState {
    pub(crate) fn scan(cwd: &Path, config: &HniConfig) -> Result<Self> {
        let discovery = ProjectDiscovery::scan(cwd, config, ScanMode::LocalBinsOnly)?;

        Ok(Self {
            ancestors: discovery.ancestors,
            bin_dirs: discovery.bin_dirs,
            detection: discovery.detection,
            has_yarn_pnp_loader: discovery.has_yarn_pnp_loader,
        })
    }

    #[must_use]
    pub(crate) fn bin_dirs(&self) -> &[PathBuf] {
        &self.bin_dirs
    }

    #[must_use]
    pub(crate) fn has_yarn_pnp_loader(&self) -> bool {
        self.has_yarn_pnp_loader
    }

    #[must_use]
    pub(crate) fn detection(&self) -> DetectionResult {
        self.detection.clone()
    }

    pub(crate) fn resolve_declared_package_bin(&self, bin_name: &str) -> Result<Option<PathBuf>> {
        resolve_declared_package_bin(&self.ancestors, bin_name)
    }
}
