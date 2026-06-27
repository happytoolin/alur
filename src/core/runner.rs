use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context, Result};

use super::{
    batch::{format_batch_command, run_batch},
    native,
    shell::shell_escape,
    types::{ExecutionStrategy, NativeExecution, ResolvedExecution},
    util::exit_code_from_status,
};
use crate::platform::node::{REAL_NODE_ENV, path_with_real_node_priority, resolve_real_node_path};

pub fn format_command(exec: &ResolvedExecution) -> Result<String> {
    if let ExecutionStrategy::InternalBatch { mode, commands } = &exec.strategy {
        return Ok(format_batch_command(*mode, commands));
    }

    if exec.is_native() {
        return Ok(native::format_command(exec));
    }

    let (program, args) = materialize(exec)?;
    let rendered = std::iter::once(shell_escape(&program))
        .chain(args.iter().map(|arg| shell_escape(arg)))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(rendered)
}

pub fn run(exec: &ResolvedExecution) -> Result<ExitCode> {
    if let ExecutionStrategy::InternalBatch { mode, commands } = &exec.strategy {
        return run_batch(*mode, commands, &exec.cwd);
    }

    if let ExecutionStrategy::Native(native_exec) = &exec.strategy {
        return match native_exec {
            NativeExecution::RunScript(script) => native::run_script(script, &exec.cwd),
            NativeExecution::RunDenoTask(task) => native::run_deno_task(task, &exec.cwd),
            NativeExecution::RunLocalBin(bin) => native::run_local_bin(bin, &exec.cwd),
            NativeExecution::RunWorkspaceScripts(scripts) => {
                native::run_workspace_scripts(scripts, &exec.cwd)
            }
            NativeExecution::RunWorkspaceLocalBins(bins) => native::run_workspace_local_bins(bins),
        };
    }

    let (program, args) = materialize(exec)?;

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

    let status = command
        .status()
        .with_context(|| format!("failed to execute {program}"))?;

    Ok(exit_code_from_status(status.code()))
}

fn materialize(exec: &ResolvedExecution) -> Result<(String, Vec<String>)> {
    let mut program = exec.program.clone();
    let args = exec.args.clone();

    if exec.passthrough && exec.program == "node" {
        let real = resolve_real_node_path()?;
        program = real.to_string_lossy().to_string();
    }

    Ok((program, args))
}
