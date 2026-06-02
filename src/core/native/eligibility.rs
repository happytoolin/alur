use anyhow::Result;

use crate::core::{
    deno::{find_nearest_deno_project, plan_native_deno_task},
    package::resolve_local_bin,
    resolve::{LocalBinProjectState, ProjectState, ResolveContext},
    types::{NativeLocalBinExecution, NativeScriptExecution, NativeScriptStep, PackageManager},
};

use super::{
    bin_resolver::resolve_local_bin_launcher,
    plan::{FallbackReason, NativeDecision, NativePlan},
};

const SUPPORTED_NPM_PACKAGE_ENV_SUFFIXES: &[&str] = &["json"];
const SUPPORTED_NPM_CONFIG_ENV_SUFFIXES: &[&str] = &["user_agent"];

pub(super) fn plan_nr_from_state(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
    state: &ProjectState,
    has_if_present: bool,
) -> Result<NativeDecision> {
    if pm == Some(PackageManager::Deno) {
        return plan_deno_nr(args, ctx, has_if_present);
    }

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

pub(super) fn plan_nlx_from_local_bin_state(
    pm: Option<PackageManager>,
    args: &[String],
    state: &LocalBinProjectState,
) -> Result<NativeDecision> {
    let Some(bin_name) = args.first() else {
        return Ok(NativeDecision::Ineligible(
            FallbackReason::MissingLocalBinCommand,
        ));
    };

    if pm == Some(PackageManager::YarnBerry) && state.has_yarn_pnp_loader() {
        return Ok(NativeDecision::Ineligible(FallbackReason::YarnBerryPnp));
    }

    let bin_paths = state.bin_dirs().to_vec();
    let bin_path = if let Some(bin_path) = resolve_local_bin(bin_name, &bin_paths) {
        Some(bin_path)
    } else if pm == Some(PackageManager::Deno) {
        None
    } else {
        state.resolve_declared_package_bin(bin_name)?
    };
    plan_local_bin(bin_name, args, bin_paths, bin_path, pm)
}

fn plan_deno_nr(
    args: &[String],
    ctx: &ResolveContext,
    has_if_present: bool,
) -> Result<NativeDecision> {
    let selection = args.first().cloned().unwrap_or_else(|| "start".to_string());
    let forwarded_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
    let Some(project) = find_nearest_deno_project(ctx.cwd())? else {
        return Ok(NativeDecision::Ineligible(
            FallbackReason::MissingNearestDenoProject,
        ));
    };

    Ok(
        match plan_native_deno_task(&project, &selection, &forwarded_args, has_if_present) {
            Ok(exec) => NativeDecision::Eligible(NativePlan::DenoTask(exec)),
            Err(reason) => NativeDecision::Ineligible(FallbackReason::DenoTask(reason.to_string())),
        },
    )
}

fn plan_local_bin(
    bin_name: &str,
    args: &[String],
    bin_paths: Vec<std::path::PathBuf>,
    bin_path: Option<std::path::PathBuf>,
    pm: Option<PackageManager>,
) -> Result<NativeDecision> {
    let Some(bin_path) = bin_path else {
        return Ok(NativeDecision::Ineligible(FallbackReason::MissingLocalBin));
    };

    Ok(NativeDecision::Eligible(NativePlan::LocalBin(
        NativeLocalBinExecution {
            bin_name: bin_name.to_string(),
            launcher: resolve_local_bin_launcher(&bin_path)?,
            forwarded_args: args.iter().skip(1).cloned().collect(),
            bin_paths,
            package_manager: pm.unwrap_or(PackageManager::Npm),
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
    if contains_unsupported_prefixed_env(script, "npm_package_", SUPPORTED_NPM_PACKAGE_ENV_SUFFIXES)
    {
        return Some("npm_package_");
    }

    if contains_unsupported_prefixed_env(script, "npm_config_", SUPPORTED_NPM_CONFIG_ENV_SUFFIXES) {
        return Some("npm_config_");
    }

    None
}

fn contains_unsupported_prefixed_env(
    script: &str,
    prefix: &str,
    supported_suffixes: &[&str],
) -> bool {
    let mut search_from = 0;
    while let Some(offset) = script[search_from..].find(prefix) {
        let prefix_start = search_from + offset + prefix.len();
        let rest = &script[prefix_start..];
        let supported = supported_suffixes.iter().any(|suffix| {
            rest.starts_with(suffix)
                && rest[suffix.len()..]
                    .chars()
                    .next()
                    .is_none_or(|ch| !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
        });
        if !supported {
            return true;
        }
        search_from = prefix_start;
    }

    false
}
