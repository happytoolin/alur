use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

mod support;

#[test]
fn init_command_renders_bash_setup() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let fake_home = dir.path().join("home");
        let fake_data = dir.path().join("data");
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_data).unwrap();

        let output = support::run_hni(
            vec!["init", "bash"],
            &[
                ("HNI_SKIP_PM_CHECK", "1"),
                ("HOME", fake_home.to_string_lossy().as_ref()),
                ("XDG_DATA_HOME", fake_data.to_string_lossy().as_ref()),
                ("LOCALAPPDATA", fake_data.to_string_lossy().as_ref()),
            ],
        );
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("# hni init"));
        assert!(stdout.contains("export PATH="));
        assert!(!stdout.contains("node() {"));
        assert!(expected_managed_node_path(&fake_home, &fake_data).exists());
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

#[cfg(not(windows))]
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
        assert!(stdout.contains("managed_node_shim:"));
        assert!(stdout.contains("node_shim_active:"));
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
        let fake_data = dir.path().join("data");

        fs::create_dir_all(&hni_bin).unwrap();
        fs::create_dir_all(&real_node_bin).unwrap();
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_data).unwrap();

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
            "eval \"$({} init bash)\"\nnode -- -v >/dev/null 2>&1\nprintf 'NODE_TYPE=%s\\nNODE_PATH=%s\\n' \"$(type -t node)\" \"$(command -v node)\"\n",
            copied_hni.display()
        );

        let expected_node = expected_managed_node_path(&fake_home, &fake_data);

        let output = Command::new(bash)
            .arg("-c")
            .arg(script)
            .env_remove("HNI_REAL_NODE")
            .env("PATH", path)
            .env("HOME", &fake_home)
            .env("XDG_DATA_HOME", &fake_data)
            .env("LOCALAPPDATA", &fake_data)
            .output()
            .expect("failed to run bash init flow");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let reported_node_type = stdout
            .lines()
            .find_map(|line| line.strip_prefix("NODE_TYPE="))
            .expect("missing NODE_TYPE line");
        let reported_node_path = stdout
            .lines()
            .find_map(|line| line.strip_prefix("NODE_PATH="))
            .expect("missing NODE_PATH line");

        assert_eq!(reported_node_type, "file");
        assert_eq!(
            Path::new(reported_node_path),
            expected_node.as_path(),
            "node should resolve to hni's managed shim path"
        );
        assert_eq!(
            std::fs::read_link(&expected_node)
                .unwrap()
                .canonicalize()
                .unwrap(),
            copied_hni.canonicalize().unwrap()
        );
    });
}

#[cfg(unix)]
#[test]
fn init_repairs_broken_and_stale_managed_node_symlinks() {
    support::with_env_lock(|| {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let hni_bin = dir.path().join("hni-bin");
        let fake_home = dir.path().join("home");
        let fake_data = dir.path().join("data");

        fs::create_dir_all(&hni_bin).unwrap();
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_data).unwrap();

        let source_exe = support::hni_executable_path();
        let first_hni = hni_bin.join("hni-first");
        let second_hni = hni_bin.join("hni-second");
        fs::copy(&source_exe, &first_hni).unwrap();
        fs::copy(&source_exe, &second_hni).unwrap();
        set_executable_if_needed(&first_hni);
        set_executable_if_needed(&second_hni);

        let managed_node = expected_managed_node_path(&fake_home, &fake_data);
        fs::create_dir_all(managed_node.parent().unwrap()).unwrap();
        symlink(dir.path().join("missing-hni"), &managed_node).unwrap();

        let first = run_init_from(&first_hni, "bash", &fake_home, &fake_data);
        assert!(first.status.success());
        assert_eq!(
            fs::read_link(&managed_node)
                .unwrap()
                .canonicalize()
                .unwrap(),
            first_hni.canonicalize().unwrap()
        );

        let second = run_init_from(&second_hni, "bash", &fake_home, &fake_data);
        assert!(second.status.success());
        assert_eq!(
            fs::read_link(&managed_node)
                .unwrap()
                .canonicalize()
                .unwrap(),
            second_hni.canonicalize().unwrap()
        );

        let rerun = run_init_from(&second_hni, "bash", &fake_home, &fake_data);
        assert!(rerun.status.success());
        assert_eq!(
            fs::read_link(&managed_node)
                .unwrap()
                .canonicalize()
                .unwrap(),
            second_hni.canonicalize().unwrap()
        );
    });
}

#[cfg(windows)]
#[test]
fn powershell_init_creates_regular_node_exe_copy() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let fake_home = dir.path().join("home");
        let fake_data = dir.path().join("data");
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_data).unwrap();

        let output = support::run_hni(
            vec!["init", "powershell"],
            &[
                ("HNI_SKIP_PM_CHECK", "1"),
                ("HOME", fake_home.to_string_lossy().as_ref()),
                ("LOCALAPPDATA", fake_data.to_string_lossy().as_ref()),
                ("APPDATA", fake_data.to_string_lossy().as_ref()),
            ],
        );
        assert!(output.status.success());

        let managed_node = expected_managed_node_path(&fake_home, &fake_data);
        let metadata = fs::symlink_metadata(&managed_node).unwrap();
        assert!(metadata.is_file());
        assert!(!metadata.file_type().is_symlink());
        assert_eq!(
            fs::read(&managed_node).unwrap(),
            fs::read(support::hni_executable_path()).unwrap()
        );
    });
}

#[cfg(windows)]
#[test]
fn powershell_init_replaces_stale_node_exe_copy() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let fake_home = dir.path().join("home");
        let fake_data = dir.path().join("data");
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_data).unwrap();

        let managed_node = expected_managed_node_path(&fake_home, &fake_data);
        fs::create_dir_all(managed_node.parent().unwrap()).unwrap();
        fs::write(&managed_node, b"stale hni launcher").unwrap();

        let output = support::run_hni(
            vec!["init", "powershell"],
            &[
                ("HNI_SKIP_PM_CHECK", "1"),
                ("HOME", fake_home.to_string_lossy().as_ref()),
                ("LOCALAPPDATA", fake_data.to_string_lossy().as_ref()),
                ("APPDATA", fake_data.to_string_lossy().as_ref()),
            ],
        );
        assert!(output.status.success());

        let metadata = fs::symlink_metadata(&managed_node).unwrap();
        assert!(metadata.is_file());
        assert!(!metadata.file_type().is_symlink());
        assert_eq!(
            fs::read(&managed_node).unwrap(),
            fs::read(support::hni_executable_path()).unwrap()
        );
    });
}

#[cfg(unix)]
#[test]
fn bash_init_keeps_package_manager_shebangs_on_real_node() {
    support::with_env_lock(|| {
        let Some(bash) = which::which("bash").ok() else {
            return;
        };

        let dir = tempfile::tempdir().unwrap();
        let hni_bin = dir.path().join("hni-bin");
        let real_node_bin = dir.path().join("real-node-bin");
        let pm_bin = dir.path().join("pm-bin");
        let fake_home = dir.path().join("home");
        let fake_config = dir.path().join("config");
        let fake_data = dir.path().join("data");

        fs::create_dir_all(&hni_bin).unwrap();
        fs::create_dir_all(&real_node_bin).unwrap();
        fs::create_dir_all(&pm_bin).unwrap();
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_config).unwrap();
        fs::create_dir_all(&fake_data).unwrap();

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
            .env("PATH", base_path)
            .env("HOME", &fake_home)
            .env("XDG_CONFIG_HOME", &fake_config)
            .env("XDG_DATA_HOME", &fake_data)
            .env("APPDATA", &fake_config)
            .env("LOCALAPPDATA", &fake_data)
            .output()
            .expect("failed to run bash version flow");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("node       v99.0.0"));
        assert!(stdout.contains("agent      npm (99.0.0)"));
        assert!(stdout.contains("global     npm (99.0.0)"));
    });
}

#[cfg(unix)]
#[test]
fn internal_real_node_path_follows_current_path_without_cache() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let first_bin = dir.path().join("first-bin");
        let second_bin = dir.path().join("second-bin");
        let fake_home = dir.path().join("home");
        let fake_data = dir.path().join("data");

        fs::create_dir_all(&first_bin).unwrap();
        fs::create_dir_all(&second_bin).unwrap();
        fs::create_dir_all(&fake_home).unwrap();
        fs::create_dir_all(&fake_data).unwrap();

        let first_node = first_bin.join("node");
        let second_node = second_bin.join("node");
        fs::write(&first_node, "#!/bin/sh\nexit 0\n").unwrap();
        fs::write(&second_node, "#!/bin/sh\nexit 0\n").unwrap();
        set_executable_if_needed(&first_node);
        set_executable_if_needed(&second_node);

        let common_env = [
            ("HOME", fake_home.to_string_lossy().to_string()),
            ("XDG_DATA_HOME", fake_data.to_string_lossy().to_string()),
        ];

        let first_path = format!(
            "{}:{}",
            first_bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let first_output = support::run_hni(
            vec!["internal", "real-node-path"],
            &[
                ("PATH", first_path.as_str()),
                ("HOME", common_env[0].1.as_str()),
                ("XDG_DATA_HOME", common_env[1].1.as_str()),
            ],
        );
        assert!(first_output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&first_output.stdout).trim(),
            first_node.to_string_lossy()
        );

        let second_path = format!(
            "{}:{}",
            second_bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let second_output = support::run_hni(
            vec!["internal", "real-node-path"],
            &[
                ("PATH", second_path.as_str()),
                ("HOME", common_env[0].1.as_str()),
                ("XDG_DATA_HOME", common_env[1].1.as_str()),
            ],
        );
        assert!(second_output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&second_output.stdout).trim(),
            second_node.to_string_lossy()
        );
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

fn expected_managed_node_path(fake_home: &Path, fake_data: &Path) -> PathBuf {
    let bin_dir = if cfg!(target_os = "macos") {
        fake_home
            .join("Library")
            .join("Application Support")
            .join("hni")
            .join("bin")
    } else {
        fake_data.join("hni").join("bin")
    };

    bin_dir.join(if cfg!(windows) { "node.exe" } else { "node" })
}

#[cfg(unix)]
fn run_init_from(
    hni: &Path,
    shell: &str,
    fake_home: &Path,
    fake_data: &Path,
) -> std::process::Output {
    Command::new(hni)
        .arg("init")
        .arg(shell)
        .env_remove("HNI_REAL_NODE")
        .env("HNI_SKIP_PM_CHECK", "1")
        .env("HOME", fake_home)
        .env("XDG_DATA_HOME", fake_data)
        .env("LOCALAPPDATA", fake_data)
        .output()
        .expect("failed to run hni init")
}
