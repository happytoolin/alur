use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

mod support;

#[test]
fn ns_stops_on_first_failure() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let cwd = work.path();
        let marker = cwd.join("should-not-exist.txt");

        let fail_cmd = failing_command(cwd, 7);
        let write_cmd = write_marker_command(&marker);

        let bin_dir = prepare_bin_dir(cwd);
        let output = run_alias_output(
            &bin_dir,
            "ns",
            vec!["-C", cwd.to_str().unwrap(), &fail_cmd, &write_cmd],
            &[],
        );

        assert!(!output.status.success());
        assert!(!marker.exists());
    });
}

#[test]
fn np_waits_for_all_commands_then_fails() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let cwd = work.path();
        let marker = cwd.join("parallel-finished.txt");

        let fail_cmd = failing_command(cwd, 5);
        let delayed_write = delayed_write_marker_command(&marker);

        let bin_dir = prepare_bin_dir(cwd);
        let output = run_alias_output(
            &bin_dir,
            "np",
            vec!["-C", cwd.to_str().unwrap(), &fail_cmd, &delayed_write],
            &[],
        );

        assert!(!output.status.success());
        assert!(marker.exists());
    });
}

#[test]
fn print_command_does_not_execute_np_or_node_p() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let cwd = work.path();

        let marker_np = cwd.join("np-print-command.txt");
        let cmd_np = write_marker_command(&marker_np);

        let marker_node = cwd.join("node-p-print-command.txt");
        let cmd_node = write_marker_command(&marker_node);

        let bin_dir = prepare_bin_dir(cwd);

        let np_output = run_alias_output(
            &bin_dir,
            "np",
            vec!["-C", cwd.to_str().unwrap(), &cmd_np, "--print-command"],
            &[],
        );
        assert!(np_output.status.success());
        let np_stdout = String::from_utf8_lossy(&np_output.stdout);
        assert!(np_stdout.contains("hni batch:parallel"));
        assert!(!marker_np.exists());

        let node_output = run_alias_output(
            &bin_dir,
            "node",
            vec![
                "-C",
                cwd.to_str().unwrap(),
                "p",
                &cmd_node,
                "--print-command",
            ],
            &[],
        );
        assert!(node_output.status.success());
        let node_stdout = String::from_utf8_lossy(&node_output.stdout);
        assert!(node_stdout.contains("hni batch:parallel"));
        assert!(!marker_node.exists());
    });
}

#[test]
fn np_and_ns_with_no_commands_succeed() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let cwd = work.path();
        let bin_dir = prepare_bin_dir(cwd);

        let np_output = run_alias_output(&bin_dir, "np", vec!["-C", cwd.to_str().unwrap()], &[]);
        assert!(np_output.status.success());
        assert!(String::from_utf8_lossy(&np_output.stdout).trim().is_empty());

        let ns_output = run_alias_output(&bin_dir, "ns", vec!["-C", cwd.to_str().unwrap()], &[]);
        assert!(ns_output.status.success());
        assert!(String::from_utf8_lossy(&ns_output.stdout).trim().is_empty());
    });
}

#[test]
fn node_parallel_and_sequential_without_commands_succeed() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let cwd = work.path();
        let bin_dir = prepare_bin_dir(cwd);

        let node_p_output = run_alias_output(
            &bin_dir,
            "node",
            vec!["-C", cwd.to_str().unwrap(), "p"],
            &[],
        );
        assert!(node_p_output.status.success());

        let node_s_output = run_alias_output(
            &bin_dir,
            "node",
            vec!["-C", cwd.to_str().unwrap(), "s"],
            &[],
        );
        assert!(node_s_output.status.success());
    });
}

fn prepare_bin_dir(cwd: &Path) -> PathBuf {
    let bin_dir = cwd.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let exe = hni_executable_path();
    if !exe.exists() {
        panic!("hni executable not found at {}", exe.display());
    }

    create_alias(&exe, &bin_dir, "np");
    create_alias(&exe, &bin_dir, "ns");
    create_alias(&exe, &bin_dir, "node");
    bin_dir
}

fn hni_executable_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_hni") {
        return PathBuf::from(path);
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push(if cfg!(windows) { "hni.exe" } else { "hni" });
    path
}

fn run_alias_output(
    bin_dir: &Path,
    alias: &str,
    args: Vec<&str>,
    extra_env: &[(&str, &str)],
) -> Output {
    let alias_bin = if cfg!(windows) {
        format!("{alias}.exe")
    } else {
        alias.to_string()
    };

    let mut cmd = Command::new(bin_dir.join(alias_bin));
    cmd.args(args).env("HNI_SKIP_PM_CHECK", "1");

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    cmd.output().expect("failed to run alias binary")
}

fn write_marker_command(path: &Path) -> String {
    let script = path.with_file_name(format!(
        "{}-write{}",
        path.file_stem().unwrap().to_string_lossy(),
        script_extension()
    ));
    let contents = if cfg!(windows) {
        format!(
            "Set-Content -LiteralPath {} -Value 'ok'\n",
            shell_quote_powershell(path)
        )
    } else {
        format!("#!/bin/sh\necho ok > {}\n", shell_quote_posix(path))
    };
    fs::write(&script, contents).unwrap();
    make_executable(&script);
    shell_command_path(&script)
}

fn delayed_write_marker_command(path: &Path) -> String {
    let script = path.with_file_name(format!(
        "{}-delayed{}",
        path.file_stem().unwrap().to_string_lossy(),
        script_extension()
    ));
    let contents = if cfg!(windows) {
        format!(
            "Start-Sleep -Milliseconds 300\nSet-Content -LiteralPath {} -Value 'ok'\n",
            shell_quote_powershell(path)
        )
    } else {
        format!(
            "#!/bin/sh\nsleep 0.2\necho ok > {}\n",
            shell_quote_posix(path)
        )
    };
    fs::write(&script, contents).unwrap();
    make_executable(&script);
    shell_command_path(&script)
}

fn failing_command(cwd: &Path, code: i32) -> String {
    let script = cwd.join(format!("fail-{code}{}", script_extension()));
    let contents = if cfg!(windows) {
        format!("exit {code}\n")
    } else {
        format!("#!/bin/sh\nexit {code}\n")
    };
    fs::write(&script, contents).unwrap();
    make_executable(&script);
    shell_command_path(&script)
}

fn shell_command_path(path: &Path) -> String {
    if cfg!(windows) {
        format!(
            "powershell -NoProfile -ExecutionPolicy Bypass -File \"{}\"",
            path.display()
        )
    } else {
        shell_quote_posix(path)
    }
}

fn shell_quote_posix(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\"'\"'"))
}

fn shell_quote_powershell(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "''"))
}

fn script_extension() -> &'static str {
    if cfg!(windows) { ".ps1" } else { ".sh" }
}

fn create_alias(target: &Path, dir: &Path, alias: &str) {
    let alias_path = if cfg!(windows) {
        dir.join(format!("{alias}.exe"))
    } else {
        dir.join(alias)
    };

    if alias_path.exists() {
        fs::remove_file(&alias_path).unwrap();
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, alias_path).unwrap();
    }

    #[cfg(windows)]
    {
        fs::copy(target, alias_path).unwrap();
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
