use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;

use crate::{
    core::types::{NativeLocalBinExecution, NativeScriptExecution, PackageManager},
    platform::node::{REAL_NODE_ENV, resolve_real_node_path},
};

pub(super) fn native_script_env(
    exec: &NativeScriptExecution,
    invocation_cwd: &std::path::Path,
) -> Result<Vec<(String, String)>> {
    let mut envs = Vec::with_capacity(7);
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

    envs.push(("npm_command".to_string(), "run-script".to_string()));

    if let Ok(user_agent) = env::var("npm_config_user_agent") {
        envs.push(("npm_config_user_agent".to_string(), user_agent));
    }

    let merged_path = merged_path_with_bins(&exec.bin_paths)?;
    envs.push(("PATH".to_string(), merged_path));
    Ok(envs)
}

pub(super) fn apply_local_bin_environment(
    command: &mut Command,
    exec: &NativeLocalBinExecution,
    invocation_cwd: &Path,
) {
    if let Ok(path) = merged_path_with_bins(&exec.bin_paths) {
        command.env("PATH", path);
    }

    if let Ok(real_node) = resolve_real_node_path() {
        command.env(REAL_NODE_ENV, &real_node);
        command.env("npm_node_execpath", real_node);
    }

    command.env("INIT_CWD", invocation_cwd);
    command.env("npm_command", "exec");
    command.env(
        "npm_execpath",
        package_manager_execpath(exec.package_manager),
    );

    if let Ok(user_agent) = env::var("npm_config_user_agent") {
        command.env("npm_config_user_agent", user_agent);
    } else {
        command.env(
            "npm_config_user_agent",
            synthetic_user_agent(exec.package_manager),
        );
    }
}

pub(super) fn merged_path_with_bins(bin_paths: &[PathBuf]) -> Result<String> {
    let current_path = env::var_os("PATH");
    let mut ordered = bin_paths.to_vec();

    if let Ok(real_node) = resolve_real_node_path()
        && let Some(real_node_dir) = real_node.parent()
    {
        ordered.push(real_node_dir.to_path_buf());
        if let Some(current_path) = current_path {
            ordered.extend(env::split_paths(&current_path).filter(|entry| entry != real_node_dir));
        }
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

fn package_manager_execpath(pm: PackageManager) -> String {
    which::which(pm.bin())
        .unwrap_or_else(|_| PathBuf::from(pm.bin()))
        .to_string_lossy()
        .to_string()
}

fn synthetic_user_agent(pm: PackageManager) -> String {
    format!("{}/0.0.0 alur/fast", pm.bin())
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, process::Command};

    use crate::core::types::{
        NativeLocalBinExecution, NativeLocalBinLauncher, NativeScriptExecution, PackageManager,
    };

    use super::{apply_local_bin_environment, merged_path_with_bins, native_script_env};

    #[test]
    fn native_script_env_sets_npm_run_contract_values() {
        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join("node_modules/.bin");
        let package_json_path = dir.path().join("package.json");
        let exec = NativeScriptExecution {
            package_root: dir.path().to_path_buf(),
            package_json_path: package_json_path.clone(),
            script_name: "build".to_string(),
            steps: Vec::new(),
            forwarded_args: Vec::new(),
            bin_paths: vec![bin_dir.clone()],
        };

        let envs = native_script_env(&exec, dir.path()).unwrap();

        assert!(envs.contains(&(
            "INIT_CWD".to_string(),
            dir.path().to_string_lossy().to_string()
        )));
        assert!(envs.contains(&(
            "npm_package_json".to_string(),
            package_json_path.to_string_lossy().to_string()
        )));
        assert!(envs.contains(&("npm_command".to_string(), "run-script".to_string())));
        assert!(
            envs.iter()
                .any(|(key, value)| key == "PATH" && value.contains(&*bin_dir.to_string_lossy()))
        );
    }

    #[test]
    fn apply_local_bin_environment_sets_exec_contract_values() {
        let dir = tempfile::tempdir().unwrap();
        let exec = NativeLocalBinExecution {
            bin_name: "demo".to_string(),
            launcher: NativeLocalBinLauncher::Binary(PathBuf::from("demo")),
            forwarded_args: Vec::new(),
            bin_paths: vec![dir.path().join("node_modules/.bin")],
            package_manager: PackageManager::Pnpm,
        };
        let mut command = Command::new("demo");

        apply_local_bin_environment(&mut command, &exec, dir.path());
        let envs = command
            .get_envs()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().to_string(),
                    value.map(|value| value.to_string_lossy().to_string()),
                )
            })
            .collect::<Vec<_>>();

        assert!(envs.contains(&(
            "INIT_CWD".to_string(),
            Some(dir.path().to_string_lossy().to_string())
        )));
        assert!(envs.contains(&("npm_command".to_string(), Some("exec".to_string()))));
        assert!(envs.iter().any(|(key, value)| {
            key == "npm_config_user_agent" && value.as_deref() == Some("pnpm/0.0.0 alur/fast")
        }));
    }

    #[test]
    fn merged_path_with_bins_keeps_requested_bins_first() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first-bin");
        let second = dir.path().join("second-bin");

        let path = merged_path_with_bins(&[first.clone(), second.clone()]).unwrap();
        let entries = std::env::split_paths(&path).collect::<Vec<_>>();

        assert_eq!(entries.first(), Some(&first));
        assert_eq!(entries.get(1), Some(&second));
    }
}
