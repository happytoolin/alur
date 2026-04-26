use std::{fs, path::Path, process::Command};

mod support;

#[test]
fn init_command_renders_bash_setup() {
    support::with_env_lock(|| {
        let output = support::run_hni(vec!["init", "bash"], &[("HNI_SKIP_PM_CHECK", "1")]);
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("# hni init for bash"));
        assert!(stdout.contains("internal real-node-path"));
        assert!(stdout.contains("export PATH="));
        assert!(stdout.contains("node() {"));
    });
}

#[test]
fn internal_real_node_path_uses_explicit_env_override() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let real_node = dir.path().join(if cfg!(windows) {
            "real-node.exe"
        } else {
            "real-node"
        });
        fs::write(&real_node, "#!/bin/sh\nexit 0\n").unwrap();
        set_executable_if_needed(&real_node);

        let output = support::run_hni(
            vec!["internal", "real-node-path"],
            &[("HNI_REAL_NODE", real_node.to_string_lossy().as_ref())],
        );
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            real_node.to_string_lossy()
        );
    });
}

#[test]
fn internal_real_node_path_succeeds_with_empty_output_when_unavailable() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let empty_path = dir.path().join("empty-bin");
        let fake_home = dir.path().join("home");
        let fake_config = dir.path().join("config");
        fs::create_dir_all(&empty_path).unwrap();
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_config).unwrap();

        let output = support::run_hni(
            vec!["internal", "real-node-path"],
            &[
                ("PATH", empty_path.to_string_lossy().as_ref()),
                ("HOME", fake_home.to_string_lossy().as_ref()),
                ("XDG_CONFIG_HOME", fake_config.to_string_lossy().as_ref()),
                ("APPDATA", fake_config.to_string_lossy().as_ref()),
            ],
        );
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).trim().is_empty());
    });
}

#[test]
fn doctor_reports_shell_setup_fields() {
    support::with_env_lock(|| {
        let output = support::run_hni(vec!["doctor"], &[("HNI_SKIP_PM_CHECK", "1")]);
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("current_hni:"));
        assert!(stdout.contains("path_node:"));
        assert!(stdout.contains("real_node:"));
        assert!(stdout.contains("shim_precedence_active:"));
    });
}

#[cfg(unix)]
#[test]
fn bash_init_gives_node_shim_precedence_and_preserves_real_node() {
    support::with_env_lock(|| {
        let Some(bash) = which::which("bash").ok() else {
            return;
        };

        let dir = tempfile::tempdir().unwrap();
        let hni_bin = dir.path().join("hni-bin");
        let real_node_bin = dir.path().join("real-node-bin");
        let fake_home = dir.path().join("home");
        let fake_config = dir.path().join("config");

        fs::create_dir_all(&hni_bin).unwrap();
        fs::create_dir_all(&real_node_bin).unwrap();
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_config).unwrap();

        let source_exe = support::hni_executable_path();
        let copied_hni = hni_bin.join("hni");
        fs::copy(&source_exe, &copied_hni).unwrap();
        set_executable_if_needed(&copied_hni);

        let fake_node = real_node_bin.join("node");
        fs::write(&fake_node, "#!/bin/sh\nexit 0\n").unwrap();
        set_executable_if_needed(&fake_node);

        let path = format!(
            "{}:{}",
            real_node_bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let script = format!(
            "eval \"$({} init bash)\"\nnode -- -v >/dev/null 2>&1\nprintf 'NODE_TYPE=%s\\nREAL=%s\\n' \"$(type -t node)\" \"$HNI_REAL_NODE\"\n",
            copied_hni.display()
        );

        let output = Command::new(bash)
            .arg("-c")
            .arg(script)
            .env_remove("HNI_REAL_NODE")
            .env_remove("HNI_NODE_SHIM_ACTIVE")
            .env("PATH", path)
            .env("HOME", &fake_home)
            .env("XDG_CONFIG_HOME", &fake_config)
            .env("APPDATA", &fake_config)
            .output()
            .expect("failed to run bash init flow");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let reported_node_type = stdout
            .lines()
            .find_map(|line| line.strip_prefix("NODE_TYPE="))
            .expect("missing NODE_TYPE line");
        let reported_real_node = stdout
            .lines()
            .find_map(|line| line.strip_prefix("REAL="))
            .expect("missing REAL line");

        assert_eq!(reported_node_type, "function");
        assert_eq!(
            Path::new(reported_real_node).canonicalize().unwrap(),
            fake_node.canonicalize().unwrap()
        );
    });
}

#[cfg(unix)]
#[test]
fn bash_init_keeps_package_manager_shebangs_on_real_node() {
    support::with_env_lock(|| {
        use std::os::unix::fs::PermissionsExt;

        let Some(bash) = which::which("bash").ok() else {
            return;
        };

        let dir = tempfile::tempdir().unwrap();
        let hni_bin = dir.path().join("hni-bin");
        let real_node_bin = dir.path().join("real-node-bin");
        let pm_bin = dir.path().join("pm-bin");
        let fake_home = dir.path().join("home");
        let fake_config = dir.path().join("config");
        let shim_log = dir.path().join("shim.log");

        fs::create_dir_all(&hni_bin).unwrap();
        fs::create_dir_all(&real_node_bin).unwrap();
        fs::create_dir_all(&pm_bin).unwrap();
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_config).unwrap();

        let source_exe = support::hni_executable_path();
        let copied_hni = hni_bin.join("hni");
        fs::copy(&source_exe, &copied_hni).unwrap();
        set_executable_if_needed(&copied_hni);

        let fake_node = real_node_bin.join("node");
        fs::write(
            &fake_node,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'v99.0.0\\n'\n  exit 0\nfi\nprintf '99.0.0\\n'\n",
        )
        .unwrap();
        set_executable_if_needed(&fake_node);

        let bad_shim_node = hni_bin.join("node");
        fs::write(
            &bad_shim_node,
            format!(
                "#!/bin/sh\nprintf 'shim-used\\n' >> '{}'\nexit 97\n",
                shim_log.display()
            ),
        )
        .unwrap();
        set_executable_if_needed(&bad_shim_node);

        let fake_npm = pm_bin.join("npm");
        fs::write(&fake_npm, "#!/usr/bin/env node\nconsole.log('npm');\n").unwrap();
        set_executable_if_needed(&fake_npm);

        let base_path = format!(
            "{}:{}:{}",
            pm_bin.display(),
            real_node_bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let script = format!(
            "eval \"$({} init bash)\"\n{} --version\n",
            copied_hni.display(),
            copied_hni.display()
        );

        let output = Command::new(bash)
            .arg("-c")
            .arg(script)
            .env_remove("HNI_REAL_NODE")
            .env_remove("HNI_NODE_SHIM_ACTIVE")
            .env("PATH", base_path)
            .env("HOME", &fake_home)
            .env("XDG_CONFIG_HOME", &fake_config)
            .env("APPDATA", &fake_config)
            .output()
            .expect("failed to run bash version flow");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("node       v99.0.0"));
        assert!(stdout.contains("agent      npm (99.0.0)"));
        assert!(stdout.contains("global     npm (99.0.0)"));
        assert!(
            !shim_log.exists()
                || fs::read_to_string(&shim_log)
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
        );

        let perms = fs::metadata(&bad_shim_node).unwrap().permissions();
        assert_eq!(perms.mode() & 0o111, 0o111);
    });
}

fn set_executable_if_needed(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).unwrap();
    }
}
