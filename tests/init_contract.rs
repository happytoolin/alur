use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

mod support;

#[test]
fn init_command_renders_bash_setup() {
    support::with_env_lock(|| {
        let home = TestHome::new();
        let output = home.run_alur(&["init", "bash"]);
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("# alur init"));
        assert!(stdout.contains("export PATH="));
        assert!(!stdout.contains("node() {"));
        assert!(home.managed_node().exists());
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

        let output = support::run_alur(
            vec!["internal", "real-node-path"],
            &[("ALUR_REAL_NODE", real_node.to_string_lossy().as_ref())],
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
fn internal_real_node_path_reports_resolution_failure_when_unavailable() {
    support::with_env_lock(|| {
        let home = TestHome::new();
        let empty_path = home.path().join("empty-bin");
        fs::create_dir_all(&empty_path).unwrap();

        let output = home
            .alur_command()
            .args(["internal", "real-node-path"])
            .env("PATH", empty_path)
            .output()
            .expect("failed to run alur");
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).trim().is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("alur: execution error"));
        assert!(stderr.contains("unable to locate real node binary"));
    });
}

#[test]
fn doctor_reports_shell_setup_fields() {
    support::with_env_lock(|| {
        let output = support::run_alur(vec!["doctor"], &[("ALUR_SKIP_PM_CHECK", "1")]);
        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("current_alur:"));
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

        let home = TestHome::new();
        let alur_bin = home.path().join("alur-bin");
        let real_node_bin = home.path().join("real-node-bin");

        fs::create_dir_all(&alur_bin).unwrap();
        fs::create_dir_all(&real_node_bin).unwrap();

        let copied_alur = copy_alur_as(&alur_bin, "alur");

        let fake_node = real_node_bin.join("node");
        write_executable(&fake_node, "#!/bin/sh\nexit 0\n");

        let expected_node = home.managed_node();
        let managed_dir = expected_node.parent().unwrap();
        let path = path_with_current(&[&real_node_bin, managed_dir]);
        let script = format!(
            "eval \"$({} init bash)\"\nnode -- -v >/dev/null 2>&1\nprintf 'NODE_TYPE=%s\\nNODE_PATH=%s\\n' \"$(type -t node)\" \"$(command -v node)\"\n",
            copied_alur.display()
        );

        let output = home
            .command(&bash)
            .arg("-c")
            .arg(script)
            .env("PATH", path)
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
            "node should resolve to alur's managed shim path"
        );
        assert_eq!(
            std::fs::read_link(&expected_node)
                .unwrap()
                .canonicalize()
                .unwrap(),
            copied_alur.canonicalize().unwrap()
        );
    });
}

#[cfg(unix)]
#[test]
fn init_repairs_broken_and_stale_managed_node_symlinks() {
    support::with_env_lock(|| {
        use std::os::unix::fs::symlink;

        let home = TestHome::new();
        let alur_bin = home.path().join("alur-bin");

        fs::create_dir_all(&alur_bin).unwrap();

        let first_alur = copy_alur_as(&alur_bin, "alur-first");
        let second_alur = copy_alur_as(&alur_bin, "alur-second");

        let managed_node = home.managed_node();
        fs::create_dir_all(managed_node.parent().unwrap()).unwrap();
        symlink(home.path().join("missing-alur"), &managed_node).unwrap();

        let first = home.run_init_from(&first_alur, "bash");
        assert!(first.status.success());
        assert_managed_node_targets(&managed_node, &first_alur);

        let second = home.run_init_from(&second_alur, "bash");
        assert!(second.status.success());
        assert_managed_node_targets(&managed_node, &second_alur);

        let rerun = home.run_init_from(&second_alur, "bash");
        assert!(rerun.status.success());
        assert_managed_node_targets(&managed_node, &second_alur);
    });
}

#[cfg(windows)]
#[test]
fn powershell_init_creates_regular_node_exe_copy() {
    support::with_env_lock(|| {
        let home = TestHome::new();
        let output = home.run_alur(&["init", "powershell"]);
        assert!(output.status.success());

        let managed_node = home.managed_node();
        let metadata = fs::symlink_metadata(&managed_node).unwrap();
        assert!(metadata.is_file());
        assert!(!metadata.file_type().is_symlink());
        assert_eq!(
            fs::read(&managed_node).unwrap(),
            fs::read(support::alur_executable_path()).unwrap()
        );
    });
}

#[cfg(windows)]
#[test]
fn powershell_init_replaces_stale_node_exe_copy() {
    support::with_env_lock(|| {
        let home = TestHome::new();
        let managed_node = home.managed_node();
        fs::create_dir_all(managed_node.parent().unwrap()).unwrap();
        fs::write(&managed_node, b"stale alur launcher").unwrap();

        let output = home.run_alur(&["init", "powershell"]);
        assert!(output.status.success());

        let metadata = fs::symlink_metadata(&managed_node).unwrap();
        assert!(metadata.is_file());
        assert!(!metadata.file_type().is_symlink());
        assert_eq!(
            fs::read(&managed_node).unwrap(),
            fs::read(support::alur_executable_path()).unwrap()
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

        let home = TestHome::new();
        let alur_bin = home.path().join("alur-bin");
        let real_node_bin = home.path().join("real-node-bin");
        let pm_bin = home.path().join("pm-bin");

        fs::create_dir_all(&alur_bin).unwrap();
        fs::create_dir_all(&real_node_bin).unwrap();
        fs::create_dir_all(&pm_bin).unwrap();

        let copied_alur = copy_alur_as(&alur_bin, "alur");

        let fake_node = real_node_bin.join("node");
        write_executable(
            &fake_node,
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  printf 'v99.0.0\\n'\n  exit 0\nfi\nprintf '99.0.0\\n'\n",
        );

        let fake_npm = pm_bin.join("npm");
        write_executable(&fake_npm, "#!/usr/bin/env node\nconsole.log('npm');\n");

        let base_path = path_with_current(&[&pm_bin, &real_node_bin]);
        let script = format!(
            "eval \"$({} init bash)\"\n{} --version\n",
            copied_alur.display(),
            copied_alur.display()
        );

        let output = home
            .command(&bash)
            .arg("-c")
            .arg(script)
            .env("PATH", base_path)
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
        let home = TestHome::new();
        let first_bin = home.path().join("first-bin");
        let second_bin = home.path().join("second-bin");

        fs::create_dir_all(&first_bin).unwrap();
        fs::create_dir_all(&second_bin).unwrap();

        let first_node = first_bin.join("node");
        let second_node = second_bin.join("node");
        write_executable(&first_node, "#!/bin/sh\nexit 0\n");
        write_executable(&second_node, "#!/bin/sh\nexit 0\n");

        let first_output = home
            .alur_command()
            .args(["internal", "real-node-path"])
            .env("PATH", path_with_current(&[&first_bin]))
            .output()
            .expect("failed to run alur");
        assert!(first_output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&first_output.stdout).trim(),
            first_node.to_string_lossy()
        );

        let second_output = home
            .alur_command()
            .args(["internal", "real-node-path"])
            .env("PATH", path_with_current(&[&second_bin]))
            .output()
            .expect("failed to run alur");
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

struct TestHome {
    dir: tempfile::TempDir,
    home: PathBuf,
    data: PathBuf,
    config: PathBuf,
}

impl TestHome {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let data = dir.path().join("data");
        let config = dir.path().join("config");

        for path in [&home, &data, &config] {
            fs::create_dir_all(path).unwrap();
        }

        Self {
            dir,
            home,
            data,
            config,
        }
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn managed_node(&self) -> PathBuf {
        expected_managed_node_path(&self.home, &self.data)
    }

    fn alur_command(&self) -> Command {
        self.command(&support::alur_executable_path())
    }

    fn command(&self, program: &Path) -> Command {
        let mut cmd = Command::new(program);
        cmd.env_remove("ALUR_CONFIG_FILE")
            .env_remove("ALUR_DEFAULT_PACKAGE_MANAGER")
            .env_remove("ALUR_GLOBAL_PACKAGE_MANAGER")
            .env_remove("ALUR_FAST_MODE")
            .env_remove("ALUR_REAL_NODE")
            .env("ALUR_SKIP_PM_CHECK", "1")
            .env("HOME", &self.home)
            .env("XDG_DATA_HOME", &self.data)
            .env("LOCALAPPDATA", &self.data)
            .env("XDG_CONFIG_HOME", &self.config)
            .env("APPDATA", &self.config);
        cmd
    }

    fn run_alur(&self, args: &[&str]) -> Output {
        self.alur_command()
            .args(args)
            .output()
            .expect("failed to run alur")
    }

    #[cfg(unix)]
    fn run_init_from(&self, alur: &Path, shell: &str) -> Output {
        self.command(alur)
            .args(["init", shell])
            .output()
            .expect("failed to run alur init")
    }
}

fn expected_managed_node_path(fake_home: &Path, fake_data: &Path) -> PathBuf {
    let bin_dir = if cfg!(target_os = "macos") {
        fake_home
            .join("Library")
            .join("Application Support")
            .join("alur")
            .join("bin")
    } else {
        fake_data.join("alur").join("bin")
    };

    bin_dir.join(if cfg!(windows) { "node.exe" } else { "node" })
}

#[cfg(unix)]
fn copy_alur_as(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    fs::copy(support::alur_executable_path(), &path).unwrap();
    set_executable_if_needed(&path);
    path
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    set_executable_if_needed(path);
}

#[cfg(unix)]
fn path_with_current(entries: &[&Path]) -> std::ffi::OsString {
    let mut paths = entries
        .iter()
        .map(|path| path.to_path_buf())
        .collect::<Vec<_>>();
    if let Some(current) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&current));
    }
    std::env::join_paths(paths).unwrap()
}

#[cfg(unix)]
fn assert_managed_node_targets(managed_node: &Path, alur: &Path) {
    assert_eq!(
        fs::read_link(managed_node).unwrap().canonicalize().unwrap(),
        alur.canonicalize().unwrap()
    );
}
