use std::path::Path;

use anyhow::{Result, anyhow};

use crate::{
    core::{
        native::{self, NativeAttempt},
        types::{ExecutionMode, Intent, PackageManager, ResolvedExecution},
    },
    platform::node::real_node_supports_run,
};

use super::{
    context::ResolveContext,
    detect::{agent_resolution_from_detection, detect_for_action, ensure_detected_available},
    flags::{exclude_flag, normalize_ni_args, prepend},
    map::{
        add_command, execute_command, frozen_command, global_install_command,
        global_uninstall_command, install_command, run_command, uninstall_command, upgrade_command,
    },
};

const NODE_RUN_UNSAFE_ENV_PATTERNS: &[&str] = &[
    "INIT_CWD",
    "npm_package_",
    "npm_config_",
    "npm_lifecycle_event",
    "npm_lifecycle_script",
    "npm_execpath",
    "npm_node_execpath",
    "npm_package_json",
];

pub fn resolve_ni(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    let use_global = args.iter().any(|arg| arg == "-g");
    let detected = detect_for_action(ctx, use_global)?;
    let args = normalize_ni_args(args, detected.pm);
    ensure_detected_available(&detected, ctx)?;

    if use_global {
        let args = exclude_flag(args, "-g");
        return Ok(build_exec(
            detected.pm,
            Intent::Install,
            args,
            ctx.cwd(),
            true,
            detected.has_lock,
        ));
    }

    if args.iter().any(|a| a == "--frozen-if-present") {
        let args = exclude_flag(args, "--frozen-if-present");
        if detected.has_lock {
            return Ok(build_exec(
                detected.pm,
                Intent::CleanInstall,
                args,
                ctx.cwd(),
                false,
                true,
            ));
        }
        return Ok(build_exec(
            detected.pm,
            Intent::Install,
            args,
            ctx.cwd(),
            false,
            false,
        ));
    }

    if args.iter().any(|a| a == "--frozen") {
        let args = exclude_flag(args, "--frozen");
        return Ok(build_exec(
            detected.pm,
            Intent::CleanInstall,
            args,
            ctx.cwd(),
            false,
            true,
        ));
    }

    if args.is_empty() || args.iter().all(|a| a.starts_with('-')) {
        return Ok(build_exec(
            detected.pm,
            Intent::Install,
            args,
            ctx.cwd(),
            false,
            detected.has_lock,
        ));
    }

    Ok(build_exec(
        detected.pm,
        Intent::Add,
        args,
        ctx.cwd(),
        false,
        detected.has_lock,
    ))
}

pub fn resolve_nr(mut args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    if args.is_empty() {
        args.push("start".to_string());
    }

    let has_if_present = args.iter().any(|a| a == "--if-present");
    if has_if_present {
        args = exclude_flag(args, "--if-present");
    }

    let mut normalized_args = args.clone();
    if normalized_args.get(1).is_some_and(|arg| arg == "--") {
        normalized_args.remove(1);
    }

    if ctx.config.fast_mode {
        let state = ctx.project_state()?;
        let detected_hint = state.detection().agent;
        match native::attempt_nr_from_state(
            detected_hint,
            &normalized_args,
            ctx,
            &state,
            has_if_present,
        )? {
            NativeAttempt::Eligible(exec) => return Ok(*exec),
            NativeAttempt::Ineligible(reason) => {
                if let Some(mut resolved) = build_node_run_exec_if_safe_from_state(
                    detected_hint,
                    &normalized_args,
                    ctx,
                    &state,
                    has_if_present,
                )? {
                    resolved.fast_requested = true;
                    resolved.fast_fallback_reason = Some(reason);
                    return Ok(resolved);
                }

                let detected = agent_resolution_from_detection(ctx, false, state.detection())?;
                ensure_detected_available(&detected, ctx)?;
                let mut resolved = build_exec(
                    detected.pm,
                    Intent::Run,
                    normalized_args,
                    ctx.cwd(),
                    false,
                    detected.has_lock,
                );

                if has_if_present {
                    insert_if_present(&mut resolved);
                }

                resolved.fast_requested = true;
                resolved.fast_fallback_reason = Some(reason);
                return Ok(resolved);
            }
        }
    }

    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;

    let mut resolved = build_exec(
        detected.pm,
        Intent::Run,
        normalized_args,
        ctx.cwd(),
        false,
        detected.has_lock,
    );

    if has_if_present {
        insert_if_present(&mut resolved);
    }

    Ok(resolved)
}

pub fn resolve_node_run(mut args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    if args.is_empty() {
        args.push("start".to_string());
    }

    let has_if_present = args.iter().any(|a| a == "--if-present");
    if has_if_present {
        args = exclude_flag(args, "--if-present");
    }

    let mut normalized_args = args.clone();
    if normalized_args.get(1).is_some_and(|arg| arg == "--") {
        normalized_args.remove(1);
    }

    if ctx.config.fast_mode {
        let state = ctx.project_state()?;
        let detected_hint = state.detection().agent;
        match native::attempt_nr_from_state(
            detected_hint,
            &normalized_args,
            ctx,
            &state,
            has_if_present,
        )? {
            NativeAttempt::Eligible(exec) => return Ok(*exec),
            NativeAttempt::Ineligible(reason) => {
                if let Some(mut resolved) = build_node_run_exec_if_safe_from_state(
                    detected_hint,
                    &normalized_args,
                    ctx,
                    &state,
                    has_if_present,
                )? {
                    resolved.fast_requested = true;
                    resolved.fast_fallback_reason = Some(reason);
                    return Ok(resolved);
                }

                let detected = agent_resolution_from_detection(ctx, false, state.detection())?;
                ensure_detected_available(&detected, ctx)?;
                let mut resolved = build_exec(
                    detected.pm,
                    Intent::Run,
                    normalized_args,
                    ctx.cwd(),
                    false,
                    detected.has_lock,
                );

                if has_if_present {
                    insert_if_present(&mut resolved);
                }

                resolved.fast_requested = true;
                resolved.fast_fallback_reason = Some(reason);
                return Ok(resolved);
            }
        }
    }

    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;

    let mut resolved = build_exec(
        detected.pm,
        Intent::Run,
        normalized_args,
        ctx.cwd(),
        false,
        detected.has_lock,
    );

    if has_if_present {
        insert_if_present(&mut resolved);
    }

    Ok(resolved)
}

pub fn resolve_nlx(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    if ctx.config.fast_mode {
        let state = ctx.project_state()?;
        let detected_hint = state.detection().agent;
        match native::attempt_nlx_from_state(detected_hint, &args, ctx, &state)? {
            NativeAttempt::Eligible(exec) => return Ok(*exec),
            NativeAttempt::Ineligible(reason) => {
                let detected = agent_resolution_from_detection(ctx, false, state.detection())?;
                ensure_detected_available(&detected, ctx)?;
                let mut resolved = build_exec(
                    detected.pm,
                    Intent::Execute,
                    args,
                    ctx.cwd(),
                    false,
                    detected.has_lock,
                );
                resolved.fast_requested = true;
                resolved.fast_fallback_reason = Some(reason);
                return Ok(resolved);
            }
        }
    }

    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;
    Ok(build_exec(
        detected.pm,
        Intent::Execute,
        args,
        ctx.cwd(),
        false,
        detected.has_lock,
    ))
}

pub fn resolve_nru(mut args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    let detected = detect_for_action(ctx, false)?;
    let interactive = args
        .iter()
        .any(|a| matches!(a.as_str(), "-i" | "--interactive"));
    if interactive {
        args = exclude_flag(args, "-i");
        args = exclude_flag(args, "--interactive");
    }

    ensure_detected_available(&detected, ctx)?;
    build_upgrade_exec(detected.pm, args, ctx.cwd(), interactive)
}

pub fn resolve_nun(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    let use_global = args.iter().any(|arg| arg == "-g");
    let detected = detect_for_action(ctx, use_global)?;
    let args = if use_global {
        exclude_flag(args, "-g")
    } else {
        args
    };

    if args.is_empty() {
        return Err(anyhow!(
            "execution error: no dependencies selected for uninstall"
        ));
    }

    ensure_detected_available(&detected, ctx)?;
    Ok(build_exec(
        detected.pm,
        Intent::Uninstall,
        args,
        ctx.cwd(),
        use_global,
        detected.has_lock,
    ))
}

pub fn resolve_nci(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;

    if detected.has_lock {
        Ok(build_exec(
            detected.pm,
            Intent::CleanInstall,
            args,
            ctx.cwd(),
            false,
            true,
        ))
    } else {
        Ok(build_exec(
            detected.pm,
            Intent::Install,
            args,
            ctx.cwd(),
            false,
            false,
        ))
    }
}

pub fn resolve_na(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;
    Ok(build_exec(
        detected.pm,
        Intent::AgentAlias,
        args,
        ctx.cwd(),
        false,
        detected.has_lock,
    ))
}

pub fn resolve_node_passthrough(args: Vec<String>, cwd: &Path) -> ResolvedExecution {
    ResolvedExecution::external_with_mode(
        "node",
        args,
        cwd.to_path_buf(),
        true,
        ExecutionMode::PassthroughNode,
    )
}

pub fn resolve_node_routed(
    intent: Intent,
    args: Vec<String>,
    ctx: &ResolveContext,
) -> Result<ResolvedExecution> {
    match intent {
        Intent::Install => resolve_ni(args, ctx),
        Intent::Add => resolve_detected_intent(intent, args, ctx),
        Intent::Execute => resolve_nlx(args, ctx),
        Intent::Run => resolve_node_run(args, ctx),
        Intent::Upgrade => resolve_nru(args, ctx),
        Intent::Uninstall => resolve_nun(args, ctx),
        Intent::CleanInstall => resolve_nci(args, ctx),
        Intent::AgentAlias => resolve_na(args, ctx),
        Intent::PassthroughNode => Ok(resolve_node_passthrough(args, ctx.cwd())),
    }
}

fn resolve_detected_intent(
    intent: Intent,
    args: Vec<String>,
    ctx: &ResolveContext,
) -> Result<ResolvedExecution> {
    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;
    Ok(build_exec(
        detected.pm,
        intent,
        args,
        ctx.cwd(),
        false,
        detected.has_lock,
    ))
}

fn build_upgrade_exec(
    pm: PackageManager,
    args: Vec<String>,
    cwd: &Path,
    interactive: bool,
) -> Result<ResolvedExecution> {
    let (program, args) = if interactive {
        match pm {
            PackageManager::Npm | PackageManager::Bun => {
                return Err(anyhow!(
                    "interactive error: interactive upgrade is not supported for {}",
                    pm.display_name()
                ));
            }
            PackageManager::Yarn => ("yarn".to_string(), prepend("upgrade-interactive", args)),
            PackageManager::YarnBerry => ("yarn".to_string(), prepend("up", prepend("-i", args))),
            PackageManager::Pnpm => ("pnpm".to_string(), prepend("update", prepend("-i", args))),
            PackageManager::Deno => (
                "deno".to_string(),
                prepend("outdated", prepend("--update", args)),
            ),
        }
    } else {
        upgrade_command(pm, args)
    };

    Ok(ResolvedExecution::external(
        program,
        args,
        cwd.to_path_buf(),
        false,
    ))
}

fn build_exec(
    pm: PackageManager,
    intent: Intent,
    args: Vec<String>,
    cwd: &Path,
    use_global: bool,
    has_lock: bool,
) -> ResolvedExecution {
    let (program, args) = match intent {
        Intent::Install => {
            if use_global {
                global_install_command(pm, args)
            } else {
                install_command(pm, args)
            }
        }
        Intent::Add => add_command(pm, args),
        Intent::Run => run_command(pm, args),
        Intent::Execute => execute_command(pm, args),
        Intent::Upgrade => upgrade_command(pm, args),
        Intent::Uninstall => {
            if use_global {
                global_uninstall_command(pm, args)
            } else {
                uninstall_command(pm, args)
            }
        }
        Intent::CleanInstall => {
            if has_lock {
                frozen_command(pm)
            } else {
                install_command(pm, args)
            }
        }
        Intent::AgentAlias => (pm.bin().to_string(), args),
        Intent::PassthroughNode => {
            return ResolvedExecution::external_with_mode(
                "node",
                args,
                cwd.to_path_buf(),
                true,
                ExecutionMode::PassthroughNode,
            );
        }
    };

    ResolvedExecution::external(program, args, cwd.to_path_buf(), false)
}

fn build_node_run_exec_if_safe_from_state(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
    state: &crate::core::resolve::ProjectState,
    has_if_present: bool,
) -> Result<Option<ResolvedExecution>> {
    if pm == Some(PackageManager::Deno) {
        return Ok(None);
    }

    if !real_node_supports_run() {
        return Ok(None);
    }

    if has_if_present {
        return Ok(None);
    }

    let Some(pkg) = state.nearest_package() else {
        return Ok(None);
    };
    let scripts = pkg.manifest.scripts.unwrap_or_default();
    let script_name = args.first().cloned().unwrap_or_else(|| "start".to_string());
    let Some(script) = scripts.get(&script_name) else {
        return Ok(None);
    };

    if scripts.contains_key(&format!("pre{script_name}"))
        || scripts.contains_key(&format!("post{script_name}"))
        || script_uses_node_run_unsupported_env(script)
        || state.has_yarn_pnp_loader()
    {
        return Ok(None);
    }

    let mut node_args = vec!["--run".to_string(), script_name];
    node_args.extend(args.iter().skip(1).cloned());

    Ok(Some(ResolvedExecution::external_with_mode(
        "node",
        node_args,
        ctx.cwd().to_path_buf(),
        true,
        ExecutionMode::NodeRun,
    )))
}

fn script_uses_node_run_unsupported_env(script: &str) -> bool {
    NODE_RUN_UNSAFE_ENV_PATTERNS
        .iter()
        .any(|pattern| script.contains(pattern))
}

fn insert_if_present(resolved: &mut ResolvedExecution) {
    if let Some(first) = resolved.args.first() {
        if matches!(first.as_str(), "run" | "task") {
            resolved.args.insert(1, "--if-present".to_string());
        } else {
            resolved.args.insert(0, "--if-present".to_string());
        }
    } else {
        resolved.args.push("--if-present".to_string());
    }
}
