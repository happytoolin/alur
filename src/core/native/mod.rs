mod bin_resolver;
mod deno;
mod eligibility;
mod env;
mod exec;
mod plan;
mod shim_parser;

use std::path::Path;

use anyhow::Result;

use crate::core::{
    resolve::{LocalBinProjectState, ProjectState, ResolveContext},
    types::{NativeDenoTaskExecution, PackageManager, ResolvedExecution},
};

use plan::{NativeDecision, NativePlan};

pub(crate) fn looks_like_env_assignment(token: &str) -> bool {
    token.contains('=') && !token.starts_with('-')
}

pub(crate) fn is_node_program(program: &str) -> bool {
    Path::new(program)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| {
            value.eq_ignore_ascii_case("node") || value.eq_ignore_ascii_case("node.exe")
        })
}

pub(crate) enum NativeAttempt {
    Eligible(Box<ResolvedExecution>),
    Ineligible(String),
}

pub(crate) fn attempt_nr_from_state(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
    state: &ProjectState,
    has_if_present: bool,
) -> Result<NativeAttempt> {
    let decision = crate::core::profile::measure("native.plan_nr", || {
        eligibility::plan_nr_from_state(pm, args, ctx, state, has_if_present)
    })?;
    Ok(crate::core::profile::measure("native.materialize", || {
        into_attempt(decision, ctx.cwd())
    }))
}

pub(crate) fn attempt_nex_from_local_bin_state(
    pm: Option<PackageManager>,
    args: &[String],
    ctx: &ResolveContext,
    state: &LocalBinProjectState,
) -> Result<NativeAttempt> {
    let decision = crate::core::profile::measure("native.plan_nex", || {
        eligibility::plan_nex_from_local_bin_state(pm, args, state)
    })?;
    Ok(crate::core::profile::measure("native.materialize", || {
        into_attempt(decision, ctx.cwd())
    }))
}

pub(crate) fn run_script(
    exec: &crate::core::types::NativeScriptExecution,
    invocation_cwd: &Path,
) -> Result<std::process::ExitCode> {
    exec::run_script(exec, invocation_cwd)
}

pub(crate) fn run_deno_task(
    exec: &NativeDenoTaskExecution,
    invocation_cwd: &Path,
) -> Result<std::process::ExitCode> {
    exec::run_deno_task(exec, invocation_cwd)
}

pub(crate) fn run_local_bin(
    exec: &crate::core::types::NativeLocalBinExecution,
    cwd: &Path,
) -> Result<std::process::ExitCode> {
    exec::run_local_bin(exec, cwd)
}

#[must_use]
pub(crate) fn format_command(exec: &ResolvedExecution) -> String {
    exec::format_command(exec)
}

fn into_attempt(decision: NativeDecision, cwd: &Path) -> NativeAttempt {
    match decision {
        NativeDecision::Eligible(plan) => NativeAttempt::Eligible(Box::new(match plan {
            NativePlan::Script(exec) => {
                ResolvedExecution::native_script(exec.script_name.clone(), cwd.to_path_buf(), exec)
            }
            NativePlan::DenoTask(exec) => {
                ResolvedExecution::native_deno_task(exec.selection.clone(), cwd.to_path_buf(), exec)
            }
            NativePlan::LocalBin(exec) => {
                ResolvedExecution::native_local_bin(exec.bin_name.clone(), cwd.to_path_buf(), exec)
            }
        })),
        NativeDecision::Ineligible(reason) => NativeAttempt::Ineligible(reason.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{is_node_program, looks_like_env_assignment};

    #[test]
    fn env_assignment_detection_requires_equals_and_non_flag_token() {
        assert!(looks_like_env_assignment("NODE_ENV=test"));
        assert!(!looks_like_env_assignment("--flag=value"));
        assert!(!looks_like_env_assignment("plain"));
    }

    #[test]
    fn node_program_detection_matches_node_binary_names_only() {
        assert!(is_node_program("node"));
        assert!(is_node_program("/usr/local/bin/node"));
        assert!(is_node_program("node.exe"));
        assert!(!is_node_program("nodejs"));
        assert!(!is_node_program("/usr/local/bin/npm"));
    }
}
