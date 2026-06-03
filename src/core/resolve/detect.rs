use std::path::PathBuf;

use anyhow::{Context, Result};
use thiserror::Error;

use crate::core::{
    detect::ensure_package_manager_available,
    types::{DetectionResult, DetectionSource, PackageManager},
};

use super::context::ResolveContext;

#[derive(Debug, Clone)]
pub(super) struct AgentResolution {
    pub pm: PackageManager,
    pub has_lock: bool,
    pub version_hint: Option<String>,
}

pub(super) fn detect_for_action(ctx: &ResolveContext, use_global: bool) -> Result<AgentResolution> {
    let config = &ctx.config;
    let detection = if use_global {
        DetectionResult {
            agent: Some(config.global_package_manager),
            has_lock: false,
            version_hint: None,
            source: DetectionSource::Config,
        }
    } else {
        ctx.detect().context("detection error")?
    };

    agent_resolution_from_detection(ctx, use_global, detection)
}

pub(super) fn agent_resolution_from_detection(
    ctx: &ResolveContext,
    use_global: bool,
    detection: DetectionResult,
) -> Result<AgentResolution> {
    let cwd = ctx.cwd();
    let pm = detection
        .agent
        .ok_or_else(|| ResolveDetectionError::MissingPackageManager {
            cwd: cwd.to_path_buf(),
        })?;

    if use_global && pm == PackageManager::YarnBerry {
        return Err(ResolveDetectionError::UnsupportedGlobalYarnBerry.into());
    }

    Ok(AgentResolution {
        pm,
        has_lock: detection.has_lock,
        version_hint: detection.version_hint,
    })
}

pub(super) fn ensure_detected_available(
    resolution: &AgentResolution,
    ctx: &ResolveContext,
) -> Result<()> {
    if !ctx.should_verify_package_manager_availability() {
        return Ok(());
    }

    ensure_package_manager_available(resolution.pm, resolution.version_hint.as_deref())
        .context("detection error")
}

#[derive(Debug, Error)]
enum ResolveDetectionError {
    #[error(
        "detection error: unable to detect package manager in {}.\nAdd packageManager to package.json, add a lockfile, or set default_package_manager in hni/config.toml",
        cwd.display()
    )]
    MissingPackageManager { cwd: PathBuf },
    #[error(
        "detection error: global install is not supported by yarn (berry).\nUse a different global_package_manager (for example: npm, pnpm, yarn, bun, deno)."
    )]
    UnsupportedGlobalYarnBerry,
}
