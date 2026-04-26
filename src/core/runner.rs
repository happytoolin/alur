use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result};

use super::{
    batch::{BatchMode, format_batch_debug, run_batch},
    native,
    shell::shell_escape,
    types::{ExecutionStrategy, NativeExecution, ResolvedExecution},
};
use crate::platform::node::{
    REAL_NODE_ENV, SHIM_ACTIVE_ENV, path_with_real_node_priority, resolve_real_node_path,
};

pub fn format_debug(exec: &ResolvedExecution) -> Result<String> {
    if let Some(mode) = BatchMode::from_internal_program(&exec.program) {
        return Ok(format_batch_debug(mode, &exec.args));
    }

    if exec.is_native() {
        return Ok(native::format_debug(exec));
    }

    let (program, args, _) = materialize(exec)?;
    let rendered = std::iter::once(shell_escape(&program))
        .chain(args.iter().map(|arg| shell_escape(arg)))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(rendered)
}

pub fn run(exec: &ResolvedExecution) -> Result<ExitCode> {
    if let Some(mode) = BatchMode::from_internal_program(&exec.program) {
        return run_batch(mode, &exec.args, &exec.cwd);
    }

    if let ExecutionStrategy::Native(native_exec) = &exec.strategy {
        return match native_exec {
            NativeExecution::RunScript(script) => native::run_script(script, &exec.cwd),
            NativeExecution::RunDenoTask(task) => native::run_deno_task(task, &exec.cwd),
            NativeExecution::RunLocalBin(bin) => native::run_local_bin(bin, &exec.cwd),
        };
    }

    let (program, args, passthrough) = materialize(exec)?;

    let mut command = Command::new(&program);
    command
        .args(&args)
        .current_dir(&exec.cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Ok(real_node) = resolve_real_node_path() {
        command.env(REAL_NODE_ENV, &real_node);
        if let Some(path) = path_with_real_node_priority(&real_node, std::env::var_os("PATH")) {
            command.env("PATH", path);
        }
    }

    if passthrough {
        command.env(SHIM_ACTIVE_ENV, "1");
    }

    let status = command
        .status()
        .with_context(|| format!("failed to execute {program}"))?;

    Ok(exit_code_from_status(status.code()))
}

fn materialize(exec: &ResolvedExecution) -> Result<(String, Vec<String>, bool)> {
    let mut program = exec.program.clone();
    let args = exec.args.clone();
    let mut passthrough = exec.passthrough;

    if exec.passthrough && exec.program == "node" {
        let real = resolve_real_node_path()?;
        program = real.to_string_lossy().to_string();
        passthrough = true;
    }

    Ok((program, args, passthrough))
}

fn exit_code_from_status(code: Option<i32>) -> ExitCode {
    code.map_or_else(|| ExitCode::from(1), exit_code_from_code)
}

fn exit_code_from_code(code: i32) -> ExitCode {
    let code = u8::try_from(code).unwrap_or(1);
    ExitCode::from(code)
}
