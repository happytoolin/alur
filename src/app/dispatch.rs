use std::process::ExitCode;

use anyhow::{Result, anyhow};

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
        detect::detect,
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
        config.clone(),
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
        } => {
            run_profile_loop(invocation, args, iterations, &resolve_ctx)?;
            Ok(ExitCode::SUCCESS)
        }
        ParsedCommand::Execute { invocation, args } => {
            let resolved = dispatch_invocation(invocation, args, &resolve_ctx)?;
            let Some(resolved) = resolved else {
                return Ok(ExitCode::SUCCESS);
            };

            if parsed.explain {
                print_explain(invocation, &resolved, &resolve_ctx, &config)?;
                return Ok(ExitCode::SUCCESS);
            }

            if parsed.debug {
                let debug_rendered = runner::format_debug(&resolved)
                    .map_err(|error| anyhow!("execution error: {error}"))?;
                println!("{debug_rendered}");
                return Ok(ExitCode::SUCCESS);
            }

            runner::run(&resolved).map_err(|error| anyhow!("execution error: {error}"))
        }
    }
}

fn print_explain(
    invocation: InvocationKind,
    resolved: &ResolvedExecution,
    ctx: &ResolveContext,
    config: &HniConfig,
) -> Result<()> {
    println!("hni explain");
    println!("invocation: {}", invocation_name(invocation));
    println!("cwd: {}", ctx.cwd().display());
    println!("fast_mode: {}", config.fast_mode);
    println!("execution_mode: {}", resolved.execution_mode_name());
    if config.fast_mode {
        let fast_status = if resolved.fast_fallback_reason.is_some() {
            "fallback"
        } else if matches!(resolved.mode, ExecutionMode::Fast | ExecutionMode::NodeRun) {
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
        runner::format_debug(resolved).map_err(|error| anyhow!("execution error: {error}"))?
    );

    if let Ok(detection) = ctx.detect().or_else(|_| detect(ctx.cwd(), &ctx.config)) {
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
    ctx: &ResolveContext,
) -> Result<()> {
    for _ in 0..iterations {
        let resolved = dispatch_invocation(invocation, args.clone(), ctx)?;
        if let Some(resolved) = resolved {
            std::hint::black_box(
                runner::format_debug(&resolved)
                    .map_err(|error| anyhow!("execution error: {error}"))?,
            );
        }
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
