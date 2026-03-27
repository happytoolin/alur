use std::{ffi::OsStr, path::Path};

use crate::{
    core::{
        config::{DefaultAgent, RunAgent},
        resolve::ResolveContext,
        types::DetectionSource,
    },
    platform::node::{real_node_supports_run, resolve_real_node_path},
};

pub fn print_doctor(ctx: &ResolveContext) {
    let cwd = ctx.cwd();
    let config = &ctx.config;
    let current_hni = std::env::current_exe()
        .ok()
        .map(|path| path.canonicalize().unwrap_or(path));
    let path_node = which::which("node").ok();
    let resolved_real_node = resolve_real_node_path().ok();

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
    println!("node_run_supported: {}", real_node_supports_run());
    println!(
        "shim_precedence_active: {}",
        shim_precedence_active(current_hni.as_deref(), path_node.as_deref())
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
        "defaultAgent: {}",
        format_default_agent(config.default_agent)
    );
    println!("globalAgent: {}", config.global_agent.display_name());
    println!("runAgent: {}", format_run_agent(config.run_agent));
    println!("nativeMode: {}", config.native_mode);
    println!("useSfw: {}", config.use_sfw);
    println!("autoInstall(env): {}", config.auto_install);
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

fn shim_precedence_active(current_hni: Option<&Path>, path_node: Option<&Path>) -> bool {
    let (Some(current_hni), Some(path_node)) = (current_hni, path_node) else {
        return false;
    };

    let Some(node_dir) = path_node.parent() else {
        return false;
    };
    let Some(hni_dir) = current_hni.parent() else {
        return false;
    };
    if !paths_equal(node_dir, hni_dir) {
        return false;
    }

    matches!(
        path_node.file_name().and_then(OsStr::to_str),
        Some("node" | "node.exe")
    )
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn format_default_agent(value: DefaultAgent) -> &'static str {
    match value {
        DefaultAgent::Prompt => "prompt",
        DefaultAgent::Agent(pm) => pm.display_name(),
    }
}

fn format_run_agent(value: RunAgent) -> &'static str {
    match value {
        RunAgent::PackageManager => "package-manager",
        RunAgent::Node => "node (legacy compatibility)",
    }
}

fn detection_source_label(value: DetectionSource) -> &'static str {
    match value {
        DetectionSource::PackageManagerField => "packageManager field",
        DetectionSource::Lockfile => "lockfile",
        DetectionSource::Config => "config defaultAgent",
        DetectionSource::DevEnginesField => "devEngines.packageManager field",
        DetectionSource::InstallMetadata => "install metadata",
        DetectionSource::Fallback => "fallback (npm in PATH)",
        DetectionSource::None => "none",
    }
}
