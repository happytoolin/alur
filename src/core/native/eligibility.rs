use anyhow::Result;

use crate::core::{
    package::resolve_local_bin,
    project::node_modules_bin_dirs,
    resolve::{LocalBinProjectState, ProjectState, ResolveContext},
    types::{
        NativeLocalBinExecution, NativeLocalBinLauncher, NativeScriptExecution, NativeScriptStep,
        NativeWorkspaceLocalBinExecution, NativeWorkspaceLocalBinPackage,
        NativeWorkspaceScriptExecution, NativeWorkspaceScriptPackage, PackageManager,
    },
    workspace::{
        WorkspacePackage, WorkspaceSelectionOptions, resolve_workspace_concurrency,
        select_workspace_packages,
    },
};

use super::{
    bin_resolver::resolve_local_bin_launcher,
    deno::{find_nearest_deno_project, plan_native_deno_task},
    plan::{FallbackReason, NativeDecision, NativePlan},
};

const SUPPORTED_NPM_PACKAGE_ENV_SUFFIXES: &[&str] = &["json", "name", "version"];
const SUPPORTED_NPM_PACKAGE_ENV_PREFIXES: &[&str] = &["config_"];
const SUPPORTED_NPM_CONFIG_ENV_SUFFIXES: &[&str] = &["user_agent", "registry"];

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

pub(super) fn plan_nex_from_local_bin_state(
    pm: Option<PackageManager>,
    args: &[String],
    state: &LocalBinProjectState,
) -> Result<NativeDecision> {
    let Some(bin_name) = args.first() else {
        return Ok(NativeDecision::Ineligible(
            FallbackReason::MissingLocalBinCommand,
        ));
    };

    let bin_paths = state.bin_dirs().to_vec();
    let bin_path = if let Some(bin_path) = resolve_local_bin(bin_name, &bin_paths) {
        Some(bin_path)
    } else if pm == Some(PackageManager::Deno) {
        None
    } else {
        state.resolve_declared_package_bin(bin_name)?
    };

    if bin_path.is_none()
        && pm == Some(PackageManager::YarnBerry)
        && is_plain_local_bin_name(bin_name)
        && let Some(pnp_loader) = state.yarn_pnp_loader()
    {
        return Ok(NativeDecision::Eligible(NativePlan::LocalBin(
            NativeLocalBinExecution {
                bin_name: bin_name.clone(),
                launcher: NativeLocalBinLauncher::YarnPnp {
                    pnp_loader: pnp_loader.to_path_buf(),
                    bin_name: bin_name.clone(),
                },
                forwarded_args: args.iter().skip(1).cloned().collect(),
                bin_paths,
                package_manager: PackageManager::YarnBerry,
            },
        )));
    }

    plan_local_bin(bin_name, args, bin_paths, bin_path, pm)
}

pub(super) fn plan_workspace_nr(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
    opts: &WorkspaceSelectionOptions,
) -> Result<NativeDecision> {
    if pm == Some(PackageManager::Deno) {
        return Ok(NativeDecision::Ineligible(FallbackReason::Workspace(
            "workspace fast mode is not supported for deno projects".to_string(),
        )));
    }

    let selection = match select_workspace_packages(ctx.cwd(), opts) {
        Ok(selection) => selection,
        Err(err) => {
            return Ok(NativeDecision::Ineligible(FallbackReason::Workspace(
                err.to_string(),
            )));
        }
    };
    let script_name = args.first().cloned().unwrap_or_else(|| "start".to_string());
    let forwarded_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
    let mut chunks = Vec::new();

    for chunk in selection.chunks {
        let mut planned = Vec::new();
        for package in chunk {
            match workspace_script_package(&package, &script_name, &forwarded_args) {
                Ok(Some(exec)) => planned.push(exec),
                Ok(None) => {}
                Err(reason) => return Ok(NativeDecision::Ineligible(reason)),
            }
        }
        if !planned.is_empty() {
            chunks.push(planned);
        }
    }

    Ok(NativeDecision::Eligible(NativePlan::WorkspaceScripts(
        NativeWorkspaceScriptExecution {
            script_name,
            chunks,
            parallel: opts.parallel,
            stream: opts.stream,
            concurrency: resolve_workspace_concurrency(opts.workspace_concurrency),
        },
    )))
}

pub(super) fn plan_workspace_nex(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
    opts: &WorkspaceSelectionOptions,
) -> Result<NativeDecision> {
    let Some(bin_name) = args.first() else {
        return Ok(NativeDecision::Ineligible(
            FallbackReason::MissingLocalBinCommand,
        ));
    };

    if pm == Some(PackageManager::Deno) {
        return Ok(NativeDecision::Ineligible(FallbackReason::Workspace(
            "workspace fast local-bin execution is not supported for deno projects".to_string(),
        )));
    }

    let selection = match select_workspace_packages(ctx.cwd(), opts) {
        Ok(selection) => selection,
        Err(err) => {
            return Ok(NativeDecision::Ineligible(FallbackReason::Workspace(
                err.to_string(),
            )));
        }
    };
    let forwarded_args = args.iter().skip(1).cloned().collect::<Vec<_>>();
    let mut chunks = Vec::new();

    for chunk in selection.chunks {
        let mut planned = Vec::new();
        for package in chunk {
            let Some(exec) = workspace_local_bin_package(pm, &package, bin_name, &forwarded_args)?
            else {
                return Ok(NativeDecision::Ineligible(FallbackReason::Workspace(
                    format!(
                        "local binary '{bin_name}' not found in workspace package '{}'",
                        package.name
                    ),
                )));
            };
            planned.push(exec);
        }
        if !planned.is_empty() {
            chunks.push(planned);
        }
    }

    Ok(NativeDecision::Eligible(NativePlan::WorkspaceLocalBins(
        NativeWorkspaceLocalBinExecution {
            bin_name: bin_name.clone(),
            chunks,
            parallel: opts.parallel,
            concurrency: resolve_workspace_concurrency(opts.workspace_concurrency),
        },
    )))
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

fn workspace_script_package(
    package: &WorkspacePackage,
    script_name: &str,
    forwarded_args: &[String],
) -> std::result::Result<Option<NativeWorkspaceScriptPackage>, FallbackReason> {
    let scripts = package.manifest.scripts.clone().unwrap_or_default();
    let Some(script) = scripts.get(script_name) else {
        return Ok(None);
    };

    let mut steps = Vec::new();
    push_step_if_present(&mut steps, &scripts, format!("pre{script_name}"), false)?;
    push_step(&mut steps, script_name.to_string(), script, true)?;
    push_step_if_present(&mut steps, &scripts, format!("post{script_name}"), false)?;

    Ok(Some(NativeWorkspaceScriptPackage {
        package_name: package.name.clone(),
        exec: NativeScriptExecution {
            package_root: package.dir.clone(),
            package_json_path: package.dir.join("package.json"),
            script_name: script_name.to_string(),
            steps,
            forwarded_args: forwarded_args.to_vec(),
            bin_paths: node_modules_bin_dirs(&package.dir),
        },
    }))
}

fn workspace_local_bin_package(
    pm: Option<PackageManager>,
    package: &WorkspacePackage,
    bin_name: &str,
    forwarded_args: &[String],
) -> Result<Option<NativeWorkspaceLocalBinPackage>> {
    let bin_paths = node_modules_bin_dirs(&package.dir);
    let bin_path = resolve_local_bin(bin_name, &bin_paths).or_else(|| {
        package
            .manifest
            .bin_command_path(bin_name)
            .map(|relative| package.dir.join(relative))
            .filter(|path| path.is_file())
    });
    let Some(bin_path) = bin_path else {
        return Ok(None);
    };

    Ok(Some(NativeWorkspaceLocalBinPackage {
        package_name: package.name.clone(),
        package_root: package.dir.clone(),
        exec: NativeLocalBinExecution {
            bin_name: bin_name.to_string(),
            launcher: resolve_local_bin_launcher(&bin_path)?,
            forwarded_args: forwarded_args.to_vec(),
            bin_paths,
            package_manager: pm.unwrap_or(PackageManager::Npm),
        },
    }))
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

fn is_plain_local_bin_name(bin_name: &str) -> bool {
    !bin_name.is_empty()
        && !bin_name
            .chars()
            .any(|ch| matches!(ch, '/' | '\\' | '@' | ':' | '\0'))
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
    if contains_unsupported_prefixed_env(
        script,
        "npm_package_",
        SUPPORTED_NPM_PACKAGE_ENV_SUFFIXES,
        SUPPORTED_NPM_PACKAGE_ENV_PREFIXES,
    ) {
        return Some("npm_package_");
    }

    if contains_unsupported_prefixed_env(
        script,
        "npm_config_",
        SUPPORTED_NPM_CONFIG_ENV_SUFFIXES,
        &[],
    ) {
        return Some("npm_config_");
    }

    None
}

fn contains_unsupported_prefixed_env(
    script: &str,
    prefix: &str,
    supported_suffixes: &[&str],
    supported_prefixes: &[&str],
) -> bool {
    let mut search_from = 0;
    while let Some(offset) = script[search_from..].find(prefix) {
        let prefix_start = search_from + offset + prefix.len();
        let rest = &script[prefix_start..];
        let supported_suffix = supported_suffixes.iter().any(|suffix| {
            rest.starts_with(suffix)
                && rest[suffix.len()..]
                    .chars()
                    .next()
                    .is_none_or(|ch| !matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
        });
        let supported_prefix = supported_prefixes
            .iter()
            .any(|supported_prefix| rest.starts_with(supported_prefix));
        if !supported_suffix && !supported_prefix {
            return true;
        }
        search_from = prefix_start;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::unsupported_pattern;

    #[test]
    fn unsupported_pattern_allows_supported_npm_env_expansions() {
        assert_eq!(unsupported_pattern("echo $npm_package_json"), None);
        assert_eq!(unsupported_pattern("echo $npm_package_name"), None);
        assert_eq!(unsupported_pattern("echo $npm_package_version"), None);
        assert_eq!(unsupported_pattern("echo $npm_package_config_port"), None);
        assert_eq!(unsupported_pattern("echo $npm_config_user_agent"), None);
        assert_eq!(unsupported_pattern("echo $npm_config_registry"), None);
    }

    #[test]
    fn unsupported_pattern_flags_unknown_npm_env_expansions() {
        assert_eq!(
            unsupported_pattern("echo $npm_package_description"),
            Some("npm_package_")
        );
        assert_eq!(
            unsupported_pattern("echo $npm_config_cache"),
            Some("npm_config_")
        );
    }
}
