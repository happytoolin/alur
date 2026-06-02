use std::path::Path;

use crate::{
    core::{resolve::ResolveContext, types::DetectionSource},
    platform::{
        node::{managed_node_shim_path, resolve_real_node_path},
        paths_equal,
    },
};

pub fn print_doctor(ctx: &ResolveContext) {
    let cwd = ctx.cwd();
    let config = &ctx.config;
    let current_hni = std::env::current_exe()
        .ok()
        .map(|path| dunce::canonicalize(&path).unwrap_or(path));
    let path_node = which::which("node").ok();
    let resolved_real_node = resolve_real_node_path().ok();
    let managed_node_shim = managed_node_shim_path();

    println!("hni doctor");
    println!();
    println!("cwd: {}", cwd.display());
    println!(
        "current_hni: {}",
        current_hni
            .as_ref()
            .map_or_else(|| "unavailable".to_string(), |p| p.display().to_string())
    );
    println!(
        "path_node: {}",
        path_node
            .as_ref()
            .map_or_else(|| "missing".to_string(), |p| p.display().to_string())
    );
    println!(
        "real_node: {}",
        resolved_real_node
            .as_ref()
            .map_or_else(|| "unavailable".to_string(), |p| p.display().to_string())
    );
    println!(
        "managed_node_shim: {}",
        managed_node_shim
            .as_ref()
            .map_or_else(|| "unavailable".to_string(), |p| p.display().to_string())
    );
    println!(
        "node_shim_active: {}",
        node_shim_active(path_node.as_deref(), managed_node_shim.as_deref())
    );
    println!();
    println!(
        "config_file: {}",
        config
            .config_path
            .as_ref()
            .map_or_else(|| "none".to_string(), |p| p.display().to_string())
    );
    println!(
        "defaultPackageManager: {}",
        config
            .default_package_manager
            .map_or("none", |pm| pm.display_name())
    );
    println!(
        "globalPackageManager: {}",
        config.global_package_manager.display_name()
    );
    println!("fastMode: {}", config.fast_mode);
    println!();

    match ctx.detect() {
        Ok(detection) => {
            println!(
                "detected_agent: {}",
                detection
                    .agent
                    .map_or_else(|| "none".to_string(), |pm| pm.display_name().to_string())
            );
            println!(
                "detection_source: {}",
                detection_source_label(detection.source)
            );
            println!("has_lockfile: {}", detection.has_lock);
            if let Some(version_hint) = detection.version_hint {
                println!("version_hint: {version_hint}");
            }
        }
        Err(err) => {
            println!("detection_error: {err}");
        }
    }

    println!();
    println!("package_manager_binaries:");
    for (label, bin) in [
        ("npm", "npm"),
        ("yarn", "yarn"),
        ("pnpm", "pnpm"),
        ("bun", "bun"),
        ("deno", "deno"),
    ] {
        let state = if which::which(bin).is_ok() {
            "ok"
        } else {
            "missing"
        };
        println!("  {label:<5} {state}");
    }
}

fn node_shim_active(path_node: Option<&Path>, managed_node_shim: Option<&Path>) -> bool {
    let (Some(path_node), Some(managed_node_shim)) = (path_node, managed_node_shim) else {
        return false;
    };

    paths_equal(path_node, managed_node_shim)
}

fn detection_source_label(value: DetectionSource) -> &'static str {
    match value {
        DetectionSource::PackageManagerField => "packageManager field",
        DetectionSource::Lockfile => "lockfile",
        DetectionSource::DevEnginesField => "devEngines.packageManager field",
        DetectionSource::InstallMetadata => "install metadata",
        DetectionSource::Config => "config defaultPackageManager",
        DetectionSource::Fallback => "fallback (npm in PATH)",
        DetectionSource::None => "none",
    }
}
