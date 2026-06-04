use std::{path::Path, process::ExitCode, thread};

use anyhow::{Context, Result};

use super::{
    shell::{configure_command, shell_command, shell_escape},
    types::{BatchMode, ResolvedExecution},
    util::{exit_code_from_code, exit_code_from_status},
};

pub fn make_execution(mode: BatchMode, commands: Vec<String>, cwd: &Path) -> ResolvedExecution {
    ResolvedExecution::internal_batch(mode, commands, cwd.to_path_buf())
}

pub fn run_batch(mode: BatchMode, commands: &[String], cwd: &Path) -> Result<ExitCode> {
    if commands.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    match mode {
        BatchMode::Sequential => run_sequential(commands, cwd),
        BatchMode::Parallel => run_parallel(commands, cwd),
    }
}

pub fn format_batch_command(mode: BatchMode, commands: &[String]) -> String {
    let mut rendered = Vec::with_capacity(commands.len() + 2);
    rendered.push("alur".to_string());
    rendered.push(format!("batch:{}", mode.label()));
    rendered.extend(commands.iter().cloned());
    rendered
        .iter()
        .map(|item| shell_escape(item))
        .collect::<Vec<_>>()
        .join(" ")
}

fn run_sequential(commands: &[String], cwd: &Path) -> Result<ExitCode> {
    for command_string in commands {
        let status = configure_command(shell_command(command_string), cwd)
            .status()
            .with_context(|| format!("failed to run command: {command_string}"))?;

        if !status.success() {
            return Ok(exit_code_from_status(status.code()));
        }
    }

    Ok(ExitCode::SUCCESS)
}

fn run_parallel(commands: &[String], cwd: &Path) -> Result<ExitCode> {
    thread::scope(|scope| {
        let handles = commands
            .iter()
            .map(|command_string| {
                let cwd = cwd.to_path_buf();
                scope.spawn(move || -> Result<i32> {
                    let status = configure_command(shell_command(command_string), &cwd)
                        .status()
                        .with_context(|| format!("failed to run command: {command_string}"))?;
                    Ok(status.code().unwrap_or(1))
                })
            })
            .collect::<Vec<_>>();

        let mut first_non_zero = None;
        for handle in handles {
            let code = handle
                .join()
                .map_err(|_| anyhow::anyhow!("parallel command worker panicked"))??;
            if code != 0 && first_non_zero.is_none() {
                first_non_zero = Some(code);
            }
        }

        Ok(first_non_zero.map_or(ExitCode::SUCCESS, exit_code_from_code))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::ExecutionStrategy;

    use std::path::PathBuf;

    #[test]
    fn formats_parallel_command() {
        let rendered = format_batch_command(
            BatchMode::Parallel,
            &["echo hello world".to_string(), "echo ok".to_string()],
        );
        assert!(rendered.starts_with("alur batch:parallel"));
        assert!(
            rendered.contains("\"echo hello world\"") || rendered.contains("'echo hello world'")
        );
    }

    #[test]
    fn formats_sequential_command() {
        let rendered = format_batch_command(BatchMode::Sequential, &["echo one".to_string()]);
        assert!(rendered.starts_with("alur batch:sequential"));
    }

    #[test]
    fn make_execution_sets_internal_batch_strategy() {
        let cwd = PathBuf::from("/tmp");
        let exec = make_execution(BatchMode::Parallel, vec!["echo hi".to_string()], &cwd);
        assert_eq!(exec.program, "alur");
        assert_eq!(exec.args, vec!["echo hi"]);
        assert_eq!(exec.cwd, cwd);
        assert_eq!(exec.execution_mode_name(), "internal");
        assert!(!exec.passthrough);
        assert!(matches!(
            exec.strategy,
            ExecutionStrategy::InternalBatch {
                mode: BatchMode::Parallel,
                ..
            }
        ));
    }

    #[test]
    fn run_batch_with_no_commands_succeeds() {
        let cwd = PathBuf::from("/tmp");
        let code_parallel = run_batch(BatchMode::Parallel, &[], &cwd).unwrap();
        let code_sequential = run_batch(BatchMode::Sequential, &[], &cwd).unwrap();
        assert_eq!(code_parallel, ExitCode::SUCCESS);
        assert_eq!(code_sequential, ExitCode::SUCCESS);
    }
}
