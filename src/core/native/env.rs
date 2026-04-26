use std::{env, path::PathBuf, process::Command};

use anyhow::Result;

use crate::{
    core::types::NativeScriptExecution,
    platform::node::{REAL_NODE_ENV, path_with_real_node_priority, resolve_real_node_path},
};

pub(super) fn native_script_env(
    exec: &NativeScriptExecution,
    invocation_cwd: &std::path::Path,
) -> Result<Vec<(String, String)>> {
    let mut envs = Vec::with_capacity(4);
    envs.push((
        "INIT_CWD".to_string(),
        invocation_cwd.to_string_lossy().to_string(),
    ));
    envs.push((
        "npm_package_json".to_string(),
        exec.package_json_path.to_string_lossy().to_string(),
    ));

    if let Ok(current_exe) = env::current_exe() {
        envs.push((
            "npm_execpath".to_string(),
            current_exe.to_string_lossy().to_string(),
        ));
    }

    if let Ok(real_node) = resolve_real_node_path() {
        envs.push((
            "npm_node_execpath".to_string(),
            real_node.to_string_lossy().to_string(),
        ));
    }

    let merged_path = merged_path_with_bins(&exec.bin_paths)?;
    envs.push(("PATH".to_string(), merged_path));
    Ok(envs)
}

pub(super) fn apply_native_environment(command: &mut Command, bin_paths: &[PathBuf]) -> Result<()> {
    if let Ok(path) = merged_path_with_bins(bin_paths) {
        command.env("PATH", path);
    }

    if let Ok(real_node) = resolve_real_node_path() {
        command.env(REAL_NODE_ENV, &real_node);
    }

    Ok(())
}

pub(super) fn merged_path_with_bins(bin_paths: &[PathBuf]) -> Result<String> {
    let current_path = env::var_os("PATH");
    let mut ordered = bin_paths.to_vec();

    if let Ok(real_node) = resolve_real_node_path()
        && let Some(path) = path_with_real_node_priority(&real_node, current_path.clone())
    {
        ordered.extend(env::split_paths(&path));
        return join_paths_string(ordered);
    }

    if let Some(current_path) = current_path {
        ordered.extend(env::split_paths(&current_path));
    }

    join_paths_string(ordered)
}

fn join_paths_string(paths: Vec<PathBuf>) -> Result<String> {
    env::join_paths(paths)
        .map(|value| value.to_string_lossy().to_string())
        .map_err(Into::into)
}
