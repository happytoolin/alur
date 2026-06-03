use std::path::Path;

use anyhow::{Result, anyhow};

use crate::core::{
    native::{self, NativeAttempt},
    types::{ExecutionMode, Intent, PackageManager, ResolvedExecution},
};

use super::{
    context::ResolveContext,
    detect::{agent_resolution_from_detection, detect_for_action, ensure_detected_available},
    flags::{exclude_flag, normalize_ni_args},
    map::{
        add_command, execute_command, frozen_command, global_install_command,
        global_uninstall_command, install_command, run_command, uninstall_command,
    },
};

/// # Errors
///
/// Fails when package-manager detection or availability checks fail.
pub fn resolve_ni(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    let use_global = args.iter().any(|arg| arg == "-g");
    let detected = detect_for_action(ctx, use_global)?;
    let args = normalize_ni_args(args, detected.pm);
    ensure_detected_available(&detected, ctx)?;

    if use_global {
        let args = exclude_flag(args, "-g");
        return Ok(build_install_exec(detected.pm, args, ctx.cwd(), true));
    }

    if args.iter().any(|a| a == "--frozen-if-present") {
        let args = exclude_flag(args, "--frozen-if-present");
        if detected.has_lock {
            Ok(build_clean_install_exec(
                detected.pm,
                args,
                ctx.cwd(),
                detected.has_lock,
            ))
        } else {
            Ok(build_install_exec(detected.pm, args, ctx.cwd(), false))
        }
    } else if args.iter().any(|a| a == "--frozen") {
        let args = exclude_flag(args, "--frozen");
        Ok(build_clean_install_exec(detected.pm, args, ctx.cwd(), true))
    } else if args.is_empty() || args.iter().all(|a| a.starts_with('-')) {
        Ok(build_install_exec(detected.pm, args, ctx.cwd(), false))
    } else {
        Ok(build_simple_intent_exec(
            detected.pm,
            SimpleIntent::Add,
            args,
            ctx.cwd(),
        ))
    }
}

/// # Errors
///
/// Fails when project scanning, package-manager detection, or native command materialization fails.
pub fn resolve_nr(mut args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    resolve_run_like(&mut args, ctx)
}

fn resolve_run_like(args: &mut Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    if args.is_empty() {
        args.push("start".to_string());
    }

    let has_if_present = args.iter().any(|a| a == "--if-present");
    if has_if_present {
        *args = exclude_flag(std::mem::take(args), "--if-present");
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
                let mut resolved =
                    build_run_fallback(ctx, normalized_args, has_if_present, state.detection())?;
                resolved.fast_requested = true;
                resolved.fast_fallback_reason = Some(reason);
                return Ok(resolved);
            }
        }
    }

    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;

    let mut resolved =
        build_simple_intent_exec(detected.pm, SimpleIntent::Run, normalized_args, ctx.cwd());

    if has_if_present {
        insert_if_present(&mut resolved);
    }

    Ok(resolved)
}

/// # Errors
///
/// Fails when local-bin scanning, package-manager detection, or availability checks fail.
pub fn resolve_nlx(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    if ctx.config.fast_mode {
        let state = ctx.local_bin_project_state()?;
        let detected_hint = state.detection().agent;
        match native::attempt_nlx_from_local_bin_state(detected_hint, &args, ctx, &state)? {
            NativeAttempt::Eligible(exec) => return Ok(*exec),
            NativeAttempt::Ineligible(reason) => {
                let detected = detect_for_action(ctx, false)?;
                ensure_detected_available(&detected, ctx)?;
                let mut resolved =
                    build_simple_intent_exec(detected.pm, SimpleIntent::Execute, args, ctx.cwd());
                resolved.fast_requested = true;
                resolved.fast_fallback_reason = Some(reason);
                return Ok(resolved);
            }
        }
    }

    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;
    Ok(build_simple_intent_exec(
        detected.pm,
        SimpleIntent::Execute,
        args,
        ctx.cwd(),
    ))
}

/// # Errors
///
/// Fails when detection fails, the command has no target dependency, or the selected package manager is unavailable.
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
            "execution error: nun requires a dependency to uninstall.\nTry: nun lodash"
        ));
    }

    ensure_detected_available(&detected, ctx)?;
    Ok(build_uninstall_exec(
        detected.pm,
        args,
        ctx.cwd(),
        use_global,
    ))
}

fn build_run_fallback(
    ctx: &ResolveContext,
    args: Vec<String>,
    has_if_present: bool,
    detection: crate::core::types::DetectionResult,
) -> Result<ResolvedExecution> {
    let detected = agent_resolution_from_detection(ctx, false, detection)?;
    ensure_detected_available(&detected, ctx)?;

    let mut resolved = build_simple_intent_exec(detected.pm, SimpleIntent::Run, args, ctx.cwd());

    if has_if_present {
        insert_if_present(&mut resolved);
    }

    Ok(resolved)
}

/// # Errors
///
/// Fails when package-manager detection or availability checks fail.
pub fn resolve_nci(args: Vec<String>, ctx: &ResolveContext) -> Result<ResolvedExecution> {
    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;

    Ok(build_clean_install_exec(
        detected.pm,
        args,
        ctx.cwd(),
        detected.has_lock,
    ))
}

pub(crate) fn resolve_node_passthrough(args: Vec<String>, cwd: &Path) -> ResolvedExecution {
    ResolvedExecution::external_with_mode(
        "node",
        args,
        cwd.to_path_buf(),
        true,
        ExecutionMode::PassthroughNode,
    )
}

pub(crate) fn resolve_node_routed(
    intent: Intent,
    args: Vec<String>,
    ctx: &ResolveContext,
) -> Result<ResolvedExecution> {
    match intent {
        Intent::Install => resolve_ni(args, ctx),
        Intent::Add => resolve_detected_intent(intent, args, ctx),
        Intent::Execute => resolve_nlx(args, ctx),
        Intent::Run => resolve_nr(args, ctx),
        Intent::Uninstall => resolve_nun(args, ctx),
        Intent::CleanInstall => resolve_nci(args, ctx),
    }
}

fn resolve_detected_intent(
    intent: Intent,
    args: Vec<String>,
    ctx: &ResolveContext,
) -> Result<ResolvedExecution> {
    let detected = detect_for_action(ctx, false)?;
    ensure_detected_available(&detected, ctx)?;
    Ok(build_simple_intent_exec(
        detected.pm,
        SimpleIntent::from_detected(intent),
        args,
        ctx.cwd(),
    ))
}

#[derive(Clone, Copy)]
enum SimpleIntent {
    Add,
    Run,
    Execute,
}

impl SimpleIntent {
    fn from_detected(intent: Intent) -> Self {
        match intent {
            Intent::Add => Self::Add,
            Intent::Run => Self::Run,
            Intent::Execute => Self::Execute,
            Intent::Install | Intent::Uninstall | Intent::CleanInstall => {
                unreachable!("policy-bearing intents use dedicated builders")
            }
        }
    }
}

fn build_install_exec(
    pm: PackageManager,
    args: Vec<String>,
    cwd: &Path,
    use_global: bool,
) -> ResolvedExecution {
    let command = if use_global {
        global_install_command(pm, args)
    } else {
        install_command(pm, args)
    };

    external_execution(command, cwd)
}

fn build_uninstall_exec(
    pm: PackageManager,
    args: Vec<String>,
    cwd: &Path,
    use_global: bool,
) -> ResolvedExecution {
    let command = if use_global {
        global_uninstall_command(pm, args)
    } else {
        uninstall_command(pm, args)
    };

    external_execution(command, cwd)
}

fn build_clean_install_exec(
    pm: PackageManager,
    args: Vec<String>,
    cwd: &Path,
    has_lock: bool,
) -> ResolvedExecution {
    let command = if has_lock {
        frozen_command(pm, args)
    } else {
        install_command(pm, args)
    };

    external_execution(command, cwd)
}

fn build_simple_intent_exec(
    pm: PackageManager,
    intent: SimpleIntent,
    args: Vec<String>,
    cwd: &Path,
) -> ResolvedExecution {
    let (program, args) = match intent {
        SimpleIntent::Add => add_command(pm, args),
        SimpleIntent::Run => run_command(pm, args),
        SimpleIntent::Execute => execute_command(pm, args),
    };

    ResolvedExecution::external(program, args, cwd.to_path_buf(), false)
}

fn external_execution(command: (String, Vec<String>), cwd: &Path) -> ResolvedExecution {
    let (program, args) = command;
    ResolvedExecution::external(program, args, cwd.to_path_buf(), false)
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
