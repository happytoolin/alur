use anyhow::{Result, anyhow};

use crate::core::{types::PackageManager, workspace::WorkspaceSelectionOptions};

#[derive(Debug, Clone, Default)]
pub(super) struct WorkspaceCommandArgs {
    pub args: Vec<String>,
    pub opts: WorkspaceSelectionOptions,
    pub requested: bool,
}

#[must_use]
pub fn exclude_flag(mut args: Vec<String>, flag: &str) -> Vec<String> {
    if let Some(pos) = args.iter().position(|arg| arg == flag) {
        args.remove(pos);
    }
    args
}

pub(super) fn normalize_ni_args(args: Vec<String>, pm: PackageManager) -> Vec<String> {
    args.into_iter()
        .map(|arg| match arg.as_str() {
            "-D" if pm == PackageManager::Bun => "-d".to_string(),
            "-P" if pm == PackageManager::Npm => "--omit=dev".to_string(),
            "-P" => "--production".to_string(),
            _ => arg,
        })
        .collect()
}

pub(super) fn split_workspace_args(args: Vec<String>) -> Result<WorkspaceCommandArgs> {
    let mut out = Vec::new();
    let mut opts = WorkspaceSelectionOptions::default();
    let mut requested = false;
    let mut idx = 0;

    while idx < args.len() {
        let arg = &args[idx];
        match arg.as_str() {
            "--" => {
                out.extend(args[idx..].iter().cloned());
                break;
            }
            "-r" | "--recursive" | "--workspaces" => {
                requested = true;
                idx += 1;
            }
            "--parallel" => {
                opts.parallel = true;
                requested = true;
                idx += 1;
            }
            "--stream" => {
                opts.stream = true;
                requested = true;
                idx += 1;
            }
            "-w" | "--workspace-root" => {
                opts.workspace_root = true;
                requested = true;
                idx += 1;
            }
            "--include-workspace-root" => {
                opts.include_workspace_root = true;
                requested = true;
                idx += 1;
            }
            "--fail-if-no-match" => {
                opts.fail_if_no_match = true;
                requested = true;
                idx += 1;
            }
            "-F" | "--filter" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| anyhow!("parse error: missing value for {arg}"))?;
                opts.filters.push(value.clone());
                requested = true;
                idx += 2;
            }
            "--workspace" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| anyhow!("parse error: missing value for --workspace"))?;
                opts.filters.push(value.clone());
                requested = true;
                idx += 2;
            }
            "--workspace-concurrency" => {
                let value = args.get(idx + 1).ok_or_else(|| {
                    anyhow!("parse error: missing value for --workspace-concurrency")
                })?;
                opts.workspace_concurrency =
                    Some(parse_i32_flag("--workspace-concurrency", value)?);
                requested = true;
                idx += 2;
            }
            "--resume-from" => {
                let value = args
                    .get(idx + 1)
                    .ok_or_else(|| anyhow!("parse error: missing value for --resume-from"))?;
                opts.resume_from = Some(value.clone());
                requested = true;
                idx += 2;
            }
            _ if arg.starts_with("--filter=") => {
                opts.filters
                    .push(arg.trim_start_matches("--filter=").to_string());
                requested = true;
                idx += 1;
            }
            _ if arg.starts_with("--workspace=") => {
                opts.filters
                    .push(arg.trim_start_matches("--workspace=").to_string());
                requested = true;
                idx += 1;
            }
            _ if arg.starts_with("--workspace-concurrency=") => {
                let value = arg.trim_start_matches("--workspace-concurrency=");
                opts.workspace_concurrency =
                    Some(parse_i32_flag("--workspace-concurrency", value)?);
                requested = true;
                idx += 1;
            }
            _ if arg.starts_with("--resume-from=") => {
                opts.resume_from = Some(arg.trim_start_matches("--resume-from=").to_string());
                requested = true;
                idx += 1;
            }
            _ if arg.starts_with("-F") && arg.len() > 2 => {
                opts.filters.push(arg[2..].to_string());
                requested = true;
                idx += 1;
            }
            _ => {
                out.extend(args[idx..].iter().cloned());
                break;
            }
        }
    }

    Ok(WorkspaceCommandArgs {
        args: out,
        opts,
        requested,
    })
}

fn parse_i32_flag(name: &str, value: &str) -> Result<i32> {
    value
        .parse()
        .map_err(|_| anyhow!("parse error: invalid value for {name}: {value}"))
}

pub(super) fn npm_run_args(args: Vec<String>) -> Vec<String> {
    if args.len() <= 1 {
        return prepend("run", args);
    }

    let mut out = vec!["run".to_string(), args[0].clone(), "--".to_string()];
    out.extend(args.into_iter().skip(1));
    out
}

pub(super) fn prepend(head: &str, mut tail: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(tail.len() + 1);
    out.push(head.to_string());
    out.append(&mut tail);
    out
}
