use anyhow::Result;

use crate::core::{
    deno::{find_nearest_deno_project, plan_native_deno_task},
    package::{node_modules_bin_dirs, resolve_local_bin},
    resolve::ResolveContext,
    types::{NativeLocalBinExecution, NativeScriptExecution, NativeScriptStep, PackageManager},
};

use super::{
    bin_resolver::resolve_local_bin_launcher,
    plan::{FallbackReason, NativeDecision, NativePlan},
};

const UNSUPPORTED_SCRIPT_PATTERNS: &[&str] = &["npm_package_", "npm_config_"];

pub(super) fn plan_nr(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
    has_if_present: bool,
) -> Result<NativeDecision> {
    if pm == Some(PackageManager::Deno) {
        let selection = args.first().cloned().unwrap_or_else(|| "start".to_string());
        let forwarded_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
        let Some(project) = find_nearest_deno_project(ctx.cwd())? else {
            return Ok(NativeDecision::Ineligible(
                FallbackReason::MissingNearestDenoProject,
            ));
        };

        return Ok(
            match plan_native_deno_task(&project, &selection, &forwarded_args, has_if_present) {
                Ok(exec) => NativeDecision::Eligible(NativePlan::DenoTask(exec)),
                Err(_) => NativeDecision::Ineligible(FallbackReason::DenoScriptExecution),
            },
        );
    }

    let state = ctx.project_state()?;
    let Some(pkg) = state.nearest_package() else {
        return Ok(NativeDecision::Ineligible(
            FallbackReason::MissingNearestPackage,
        ));
    };

    if pm == Some(PackageManager::YarnBerry) && state.has_yarn_pnp_loader() {
        return Ok(NativeDecision::Ineligible(FallbackReason::YarnBerryPnp));
    }

    let scripts = pkg.manifest.scripts.unwrap_or_default();
    let script_name = args.first().cloned().unwrap_or_else(|| "start".to_string());
    let forwarded_args = args.iter().skip(1).cloned().collect::<Vec<_>>();

    let Some(script) = scripts.get(&script_name) else {
        if has_if_present {
            return Ok(NativeDecision::Eligible(NativePlan::Script(
                NativeScriptExecution {
                    package_root: pkg.root,
                    package_json_path: pkg.package_json_path,
                    script_name,
                    steps: Vec::new(),
                    forwarded_args,
                    bin_paths: state.bin_dirs().to_vec(),
                },
            )));
        }

        return Ok(NativeDecision::Ineligible(FallbackReason::MissingScript(
            script_name,
        )));
    };

    let mut steps = Vec::new();
    if let Err(reason) =
        push_step_if_present(&mut steps, &scripts, format!("pre{script_name}"), false)
    {
        return Ok(NativeDecision::Ineligible(reason));
    }
    if let Err(reason) = push_step(&mut steps, script_name.clone(), script, true) {
        return Ok(NativeDecision::Ineligible(reason));
    }
    if let Err(reason) =
        push_step_if_present(&mut steps, &scripts, format!("post{script_name}"), false)
    {
        return Ok(NativeDecision::Ineligible(reason));
    }

    Ok(NativeDecision::Eligible(NativePlan::Script(
        NativeScriptExecution {
            package_root: pkg.root,
            package_json_path: pkg.package_json_path,
            script_name,
            steps,
            forwarded_args,
            bin_paths: state.bin_dirs().to_vec(),
        },
    )))
}

pub(super) fn plan_nlx(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
) -> Result<NativeDecision> {
    let Some(bin_name) = args.first() else {
        return Ok(NativeDecision::Ineligible(
            FallbackReason::MissingLocalBinCommand,
        ));
    };

    if pm == Some(PackageManager::Deno) {
        let bin_paths = node_modules_bin_dirs(ctx.cwd());
        let Some(bin_path) = resolve_local_bin(bin_name, &bin_paths) else {
            return Ok(NativeDecision::Ineligible(FallbackReason::RemoteDenoExec));
        };

        return Ok(NativeDecision::Eligible(NativePlan::LocalBin(
            NativeLocalBinExecution {
                bin_name: bin_name.clone(),
                launcher: resolve_local_bin_launcher(&bin_path)?,
                forwarded_args: args.iter().skip(1).cloned().collect(),
                bin_paths,
            },
        )));
    }

    let state = ctx.project_state()?;

    if pm == Some(PackageManager::YarnBerry) && state.has_yarn_pnp_loader() {
        return Ok(NativeDecision::Ineligible(FallbackReason::YarnBerryPnp));
    }

    let bin_paths = state.bin_dirs().to_vec();
    let bin_path = resolve_local_bin(bin_name, &bin_paths)
        .or_else(|| state.resolve_declared_package_bin(bin_name));
    let Some(bin_path) = bin_path else {
        return Ok(NativeDecision::Ineligible(FallbackReason::MissingLocalBin));
    };

    Ok(NativeDecision::Eligible(NativePlan::LocalBin(
        NativeLocalBinExecution {
            bin_name: bin_name.clone(),
            launcher: resolve_local_bin_launcher(&bin_path)?,
            forwarded_args: args.iter().skip(1).cloned().collect(),
            bin_paths,
        },
    )))
}

fn push_step_if_present(
    steps: &mut Vec<NativeScriptStep>,
    scripts: &std::collections::BTreeMap<String, String>,
    event_name: String,
    forward_args: bool,
) -> std::result::Result<(), FallbackReason> {
    let Some(command) = scripts.get(&event_name) else {
        return Ok(());
    };

    push_step(steps, event_name, command, forward_args)
}

fn push_step(
    steps: &mut Vec<NativeScriptStep>,
    event_name: String,
    command: &str,
    forward_args: bool,
) -> std::result::Result<(), FallbackReason> {
    if let Some(pattern) = unsupported_pattern(command) {
        return Err(FallbackReason::UnsupportedScriptEnv {
            event_name,
            pattern,
        });
    }

    steps.push(NativeScriptStep {
        event_name,
        command: command.to_string(),
        forward_args,
    });
    Ok(())
}

fn unsupported_pattern(script: &str) -> Option<&'static str> {
    UNSUPPORTED_SCRIPT_PATTERNS
        .iter()
        .find(|pattern| script.contains(**pattern))
        .copied()
}
