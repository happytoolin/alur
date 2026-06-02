use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};

use crate::{
    app::{
        cli::{ParsedCommand, parse_from_env},
        command_registry::command_spec_by_invocation,
        completion::print_completion,
        doctor::print_doctor,
        help::print_help,
        init::print_init,
        version::print_versions,
    },
    core::{
        config::HniConfig,
        resolve::ResolveContext,
        runner,
        types::{ExecutionMode, InvocationKind, ResolvedExecution},
    },
    platform::node::resolve_real_node_path,
};

pub fn run_from_env() -> Result<ExitCode> {
    let parsed = parse_from_env()?;
    if parsed.deprecated_debug_alias_used {
        eprintln!(
            "[hni] warning: '?' debug alias is deprecated; use --debug-resolved, --dry-run, or --print-command"
        );
    }

    if !parsed.cwd.exists() {
        return Err(anyhow!(
            "execution error: working directory does not exist: {}",
            parsed.cwd.display()
        ));
    }

    let mut config = HniConfig::load()?;
    if let Some(fast_override) = parsed.fast_override {
        config.fast_mode = fast_override;
    }
    let verify_package_manager_availability =
        matches!(&parsed.command, ParsedCommand::Execute { .. })
            && !parsed.debug
            && !parsed.explain;
    let resolve_ctx = ResolveContext::with_package_manager_checks(
        parsed.cwd.clone(),
        config,
        verify_package_manager_availability,
    );

    match parsed.command {
        ParsedCommand::PrintVersions => {
            print_versions(&resolve_ctx);
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::PrintHelp(topic) => {
            print_help(topic);
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::Doctor => {
            print_doctor(&resolve_ctx);
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::Completion { shell, program } => {
            print_completion(shell.as_deref(), &program)?;
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::Init { shell } => {
            print_init(&shell)?;
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::InternalRealNodePath => {
            if let Ok(path) = resolve_real_node_path() {
                println!("{}", path.display());
            }
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::InternalProfileLoop {
            invocation,
            args,
            iterations,
            timings,
        } => {
            run_profile_loop(invocation, args, iterations, timings, &resolve_ctx)?;
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::Execute { invocation, args } => {
            let resolved = dispatch_invocation(invocation, args, &resolve_ctx)?;
            let Some(resolved) = resolved else {
                return Ok(ExitCode::SUCCESS);
            };

            if parsed.explain {
                print_explain(invocation, &resolved, &resolve_ctx)?;
                return Ok(ExitCode::SUCCESS);
            }

            if parsed.debug {
                let debug_rendered = runner::format_debug(&resolved).context("execution error")?;
                println!("{debug_rendered}");
                return Ok(ExitCode::SUCCESS);
            }

            runner::run(&resolved).context("execution error")
        }
    }
}

fn print_explain(
    invocation: InvocationKind,
    resolved: &ResolvedExecution,
    ctx: &ResolveContext,
) -> Result<()> {
    println!("hni explain");
    println!("invocation: {}", invocation_name(invocation));
    println!("cwd: {}", ctx.cwd().display());
    println!("fast_mode: {}", ctx.config.fast_mode);
    println!("execution_mode: {}", resolved.execution_mode_name());
    if ctx.config.fast_mode {
        let fast_status = if resolved.fast_fallback_reason.is_some() {
            "fallback"
        } else if matches!(resolved.mode, ExecutionMode::Fast) {
            "eligible"
        } else {
            "not-applicable"
        };
        println!("fast_status: {}", fast_status);
        if let Some(reason) = &resolved.fast_fallback_reason {
            println!("fast_fallback_reason: {reason}");
        }
    }
    println!(
        "resolved: {}",
        runner::format_debug(resolved).context("execution error")?
    );

    if let Ok(detection) = ctx.detect() {
        println!(
            "detected_agent: {}",
            detection
                .agent
                .map_or_else(|| "none".to_string(), |pm| pm.display_name().to_string())
        );
        println!("detection_source: {:?}", detection.source);
        println!("has_lockfile: {}", detection.has_lock);
    }

    Ok(())
}

fn run_profile_loop(
    invocation: InvocationKind,
    args: Vec<String>,
    iterations: usize,
    timings: bool,
    ctx: &ResolveContext,
) -> Result<()> {
    if timings {
        crate::core::profile::start();
    }

    for _ in 0..iterations {
        let resolved = crate::core::profile::measure("dispatch.resolve", || {
            dispatch_invocation(invocation, args.clone(), ctx)
        })?;
        if let Some(resolved) = resolved {
            std::hint::black_box(crate::core::profile::measure(
                "runner.format_debug",
                || runner::format_debug(&resolved).context("execution error"),
            )?);
        }
    }

    if timings && let Some(rendered) = crate::core::profile::finish(iterations) {
        println!("{rendered}");
    }

    Ok(())
}

fn invocation_name(invocation: InvocationKind) -> &'static str {
    command_spec_by_invocation(invocation)
        .map(|spec| spec.name)
        .unwrap_or("hni")
}

fn dispatch_invocation(
    invocation: InvocationKind,
    args: Vec<String>,
    ctx: &ResolveContext,
) -> Result<Option<ResolvedExecution>> {
    let Some(spec) = command_spec_by_invocation(invocation) else {
        return Err(anyhow!("execution error: missing command"));
    };

    (spec.handler)(args, ctx)
}
