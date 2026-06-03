use std::fs;

mod support;

use support::run_hni;

#[test]
fn help_and_version_contracts_are_hni_first() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"x"}"#).unwrap();

        let help_subcommand = run_hni(vec!["help", "ni"], &[("HNI_SKIP_PM_CHECK", "1")]);
        assert!(help_subcommand.status.success());
        let help_subcommand_out = String::from_utf8_lossy(&help_subcommand.stdout);
        assert!(help_subcommand_out.contains("Usage: ni"));

        let help_flag = run_hni(
            vec!["install", "-C", project.to_str().unwrap(), "--help"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(help_flag.status.success());
        let help_flag_out = String::from_utf8_lossy(&help_flag.stdout);
        assert!(help_flag_out.contains("Usage: ni"));
        assert!(!help_flag_out.contains("Usage:\nnpm install"));

        let passthrough_help = run_hni(
            vec![
                "install",
                "-C",
                project.to_str().unwrap(),
                "--print-command",
                "--",
                "--help",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(passthrough_help.status.success());
        let passthrough_help_out = String::from_utf8_lossy(&passthrough_help.stdout);
        assert_eq!(passthrough_help_out.trim(), "npm i --help");

        let version = run_hni(
            vec!["install", "-C", project.to_str().unwrap(), "--version"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(version.status.success());
        let version_out = String::from_utf8_lossy(&version.stdout);
        assert!(version_out.contains("hni       v"));
    });
}

#[test]
fn global_flags_work_anywhere_before_passthrough_separator() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"x"}"#).unwrap();

        let output = run_hni(
            vec![
                "install",
                "-C",
                project.to_str().unwrap(),
                "vite",
                "--print-command",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "npm i vite");
    });
}

#[test]
fn fast_and_pm_cli_flags_override_environment_setting() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        fs::create_dir_all(project.join("node_modules").join(".bin")).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();
        fs::write(project.join("node_modules").join(".bin").join("vite"), "").unwrap();

        let force_fast = run_hni(
            vec![
                "run",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "--print-command",
                "dev",
            ],
            &[("HNI_SKIP_PM_CHECK", "1"), ("HNI_FAST_MODE", "false")],
        );
        assert!(force_fast.status.success());
        assert_eq!(
            String::from_utf8_lossy(&force_fast.stdout).trim(),
            "hni fast:run-script dev"
        );

        let force_pm = run_hni(
            vec![
                "run",
                "-C",
                project.to_str().unwrap(),
                "--pm",
                "--print-command",
                "dev",
            ],
            &[("HNI_SKIP_PM_CHECK", "1"), ("HNI_FAST_MODE", "true")],
        );
        assert!(force_pm.status.success());
        assert_eq!(
            String::from_utf8_lossy(&force_pm.stdout).trim(),
            "npm run dev"
        );
    });
}

#[test]
fn default_fast_mode_resolves_nr_and_nlx_natively() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        let bin_dir = project.join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();
        fs::write(bin_dir.join("vite"), "").unwrap();
        fs::write(bin_dir.join("hello"), "#!/bin/sh\nexit 0\n").unwrap();
        make_executable(&bin_dir.join("hello"));

        support::with_var_removed("HNI_FAST_MODE", || {
            let nr = run_hni(
                vec![
                    "run",
                    "-C",
                    project.to_str().unwrap(),
                    "--print-command",
                    "dev",
                ],
                &[("HNI_SKIP_PM_CHECK", "1")],
            );
            assert!(nr.status.success(), "{nr:?}");
            assert_eq!(
                String::from_utf8_lossy(&nr.stdout).trim(),
                "hni fast:run-script dev"
            );

            let nlx = run_hni(
                vec![
                    "exec",
                    "-C",
                    project.to_str().unwrap(),
                    "--print-command",
                    "hello",
                    "world",
                ],
                &[("HNI_SKIP_PM_CHECK", "1")],
            );
            assert!(nlx.status.success(), "{nlx:?}");
            assert_eq!(
                String::from_utf8_lossy(&nlx.stdout).trim(),
                "hni fast:run-local-bin hello world"
            );
        });
    });
}

#[test]
fn fast_flag_enables_fast_mode() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        fs::create_dir_all(project.join("node_modules").join(".bin")).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();
        fs::write(project.join("node_modules").join(".bin").join("vite"), "").unwrap();

        let output = run_hni(
            vec![
                "run",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "--print-command",
                "dev",
            ],
            &[("HNI_SKIP_PM_CHECK", "1"), ("HNI_FAST_MODE", "false")],
        );
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "hni fast:run-script dev"
        );
    });
}

#[test]
fn internal_profile_loop_resolves_commands_without_running_them() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        fs::create_dir_all(project.join("node_modules").join(".bin")).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();
        fs::write(project.join("node_modules").join(".bin").join("vite"), "").unwrap();

        let output = run_hni(
            vec![
                "internal",
                "profile-loop",
                "--iterations",
                "3",
                "nr",
                "dev",
                "-C",
                project.to_str().unwrap(),
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).trim().is_empty());

        let np = run_hni(
            vec![
                "internal",
                "profile-loop",
                "--iterations",
                "2",
                "np",
                "echo hi",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(np.status.success(), "{np:?}");

        let ns = run_hni(
            vec![
                "internal",
                "profile-loop",
                "--iterations",
                "2",
                "ns",
                "echo hi",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(ns.status.success(), "{ns:?}");
    });
}

#[test]
fn print_command_and_explain_skip_package_manager_availability_checks() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("pnpm");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("pnpm-lock.yaml"), "lock").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"x"}"#).unwrap();

        let printed = run_hni(
            vec![
                "install",
                "-C",
                project.to_str().unwrap(),
                "--print-command",
                "react",
            ],
            &[("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")],
        );
        assert!(printed.status.success(), "{printed:?}");
        assert_eq!(
            String::from_utf8_lossy(&printed.stdout).trim(),
            "pnpm add react"
        );

        let explain = run_hni(
            vec![
                "install",
                "-C",
                project.to_str().unwrap(),
                "--explain",
                "react",
            ],
            &[("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")],
        );
        assert!(explain.status.success(), "{explain:?}");
        let stdout = String::from_utf8_lossy(&explain.stdout);
        assert!(stdout.contains("hni explain"));
        assert!(stdout.contains("resolved: pnpm add react"));
    });
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}
