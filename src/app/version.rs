use std::{path::Path, process::Command, thread};

use crate::{
    core::{
        detect::detect,
        resolve::{ResolveContext, version_command_for_pm},
    },
    platform::node::{REAL_NODE_ENV, path_with_real_node_priority, resolve_real_node_path},
};

pub fn print_versions(ctx: &ResolveContext) {
    println!("alur       v{}", env!("CARGO_PKG_VERSION"));

    let real_node = resolve_real_node_path().ok();
    let cwd = ctx.cwd().to_path_buf();
    let detected_cwd = cwd.clone();
    let detected_config = ctx.config.clone();
    let global = ctx.config.global_package_manager;

    let node_version_task = {
        let cwd = cwd.clone();
        let real_node = real_node.clone();
        thread::spawn(move || {
            real_node.as_ref().and_then(|path| {
                run_version(
                    path.to_string_lossy().to_string(),
                    vec!["--version".into()],
                    &cwd,
                    real_node.as_deref(),
                )
            })
        })
    };

    let detected_agent_task = {
        let real_node = real_node.clone();
        thread::spawn(move || {
            detect(&detected_cwd, &detected_config)
                .ok()
                .and_then(|detected| detected.agent.map(|agent| (agent, detected.source)))
                .map(|(agent, _)| {
                    let (program, args) = version_command_for_pm(agent);
                    let version = run_version(program, args, &detected_cwd, real_node.as_deref());
                    (agent, version)
                })
        })
    };

    let global_version_task = {
        let cwd = cwd.clone();
        let real_node = real_node.clone();
        thread::spawn(move || {
            let (program, args) = version_command_for_pm(global);
            run_version(program, args, &cwd, real_node.as_deref())
        })
    };

    let node_version = node_version_task.join().ok().flatten();
    if let Some(version) = node_version {
        println!("node       {version}");
    }

    match detected_agent_task.join().ok().flatten() {
        Some((agent, Some(version))) => {
            println!("agent      {} ({version})", agent.display_name());
        }
        Some((agent, None)) => {
            println!("agent      {}", agent.display_name());
        }
        None => {
            println!("agent      none");
        }
    }

    if let Some(version) = global_version_task.join().ok().flatten() {
        println!("global     {} ({version})", global.display_name());
    } else {
        println!("global     {}", global.display_name());
    }
}

fn run_version(
    program: String,
    args: Vec<String>,
    cwd: &Path,
    real_node: Option<&Path>,
) -> Option<String> {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd);

    if let Some(real_node) = real_node {
        command.env(REAL_NODE_ENV, real_node);
        if let Some(path) = path_with_real_node_priority(real_node, std::env::var_os("PATH")) {
            command.env("PATH", path);
        }
    }

    let output = command.output().ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::run_version;

    #[cfg(unix)]
    #[test]
    fn run_version_returns_trimmed_stdout_for_successful_commands() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("version-tool");
        std::fs::write(&script, "#!/bin/sh\nprintf 'v1.2.3\\n'\n").unwrap();
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).unwrap();

        assert_eq!(
            run_version(
                script.to_string_lossy().to_string(),
                Vec::new(),
                dir.path(),
                None
            ),
            Some("v1.2.3".to_string())
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_version_returns_none_for_failed_or_empty_commands() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty-tool");
        let fail = dir.path().join("fail-tool");
        std::fs::write(&empty, "#!/bin/sh\n").unwrap();
        std::fs::write(&fail, "#!/bin/sh\nexit 1\n").unwrap();

        for script in [&empty, &fail] {
            let mut permissions = std::fs::metadata(script).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(script, permissions).unwrap();
        }

        assert_eq!(
            run_version(
                empty.to_string_lossy().to_string(),
                Vec::new(),
                dir.path(),
                None
            ),
            None
        );
        assert_eq!(
            run_version(
                fail.to_string_lossy().to_string(),
                Vec::new(),
                dir.path(),
                None
            ),
            None
        );
    }
}
