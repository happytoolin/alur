#![cfg(unix)]

use std::fs;

mod support;

use support::run_hni;

#[test]
fn native_nr_runs_hooks_from_nearest_package_and_forwards_args() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().join("workspace");
        let pkg = root.join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(root.join("package-lock.json"), "lock").unwrap();
        fs::write(root.join("package.json"), r#"{"name":"workspace"}"#).unwrap();
        fs::write(
            pkg.join("write-args.cjs"),
            "const fs = require('fs'); fs.writeFileSync('args.txt', JSON.stringify(process.argv.slice(2))); fs.appendFileSync('order.txt', 'dev');\n",
        )
        .unwrap();
        fs::write(
            pkg.join("package.json"),
            r#"{"name":"app","scripts":{"predev":"printf 'pre' >> order.txt","dev":"node write-args.cjs","postdev":"printf 'post' >> order.txt"}}"#,
        )
        .unwrap();

        let output = run_hni(
            vec![
                "nr",
                "-C",
                pkg.to_str().unwrap(),
                "--fast",
                "dev",
                "--",
                "alpha",
                "beta",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            fs::read_to_string(pkg.join("order.txt")).unwrap(),
            "predevpost"
        );
        assert_eq!(
            fs::read_to_string(pkg.join("args.txt")).unwrap(),
            "[\"alpha\",\"beta\"]"
        );
    });
}

#[test]
fn native_nlx_runs_local_bin_directly() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        let bin_dir = project.join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"x"}"#).unwrap();

        let bin = bin_dir.join("hello");
        fs::write(&bin, "#!/bin/sh\nprintf '%s' \"$*\" > bin-args.txt\n").unwrap();
        make_executable(&bin);

        let output = run_hni(
            vec![
                "nlx",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "hello",
                "world",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            fs::read_to_string(project.join("bin-args.txt")).unwrap(),
            "world"
        );
    });
}

#[test]
fn native_explain_reports_fallback_reason() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"echo $npm_package_name"}}"#,
        )
        .unwrap();

        let output = run_hni(
            vec![
                "nr",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "--explain",
                "dev",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("fast_mode: true"));
        assert!(stdout.contains("execution_mode: package-manager"));
        assert!(stdout.contains("fast_status: fallback"));
        assert!(stdout.contains("fast_fallback_reason:"));
        assert!(stdout.contains("resolved: npm run dev"));
    });
}

#[test]
fn native_nr_preserves_shell_glob_expansion() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        let src_dir = project.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"show":"printf \"%s\n\" src/*.js"}}"#,
        )
        .unwrap();
        fs::write(src_dir.join("a.js"), "").unwrap();
        fs::write(src_dir.join("b.js"), "").unwrap();

        let output = run_hni(
            vec!["nr", "-C", project.to_str().unwrap(), "--fast", "show"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "src/a.js\nsrc/b.js\n"
        );
    });
}

#[test]
fn node_run_prefers_builtin_node_run_when_supported() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        let fake_node = work.path().join("fake-node");
        let bin_dir = project.join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"echo ok"}}"#,
        )
        .unwrap();
        fs::write(
            &fake_node,
            "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  printf '  --run\\n'\n  exit 0\nfi\nexit 0\n",
        )
        .unwrap();
        make_executable(&fake_node);

        let run_output = run_hni(
            vec![
                "node",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "--debug-resolved",
                "run",
                "dev",
            ],
            &[
                ("HNI_SKIP_PM_CHECK", "1"),
                ("HNI_REAL_NODE", fake_node.to_str().unwrap()),
            ],
        );
        assert!(run_output.status.success(), "{run_output:?}");
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let rendered = stdout.trim();
        assert!(rendered.contains(fake_node.to_str().unwrap()), "{rendered}");
        assert!(rendered.ends_with(" --run dev"), "{rendered}");
    });
}

#[test]
fn node_run_falls_back_to_native_when_node_run_is_unsafe() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        let fake_node = work.path().join("fake-node");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"predev":"echo pre","dev":"echo ok"}}"#,
        )
        .unwrap();
        fs::write(
            &fake_node,
            "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  printf '  --run\\n'\n  exit 0\nfi\nexit 0\n",
        )
        .unwrap();
        make_executable(&fake_node);

        let run_output = run_hni(
            vec![
                "node",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "--debug-resolved",
                "run",
                "dev",
            ],
            &[
                ("HNI_SKIP_PM_CHECK", "1"),
                ("HNI_REAL_NODE", fake_node.to_str().unwrap()),
            ],
        );
        assert!(run_output.status.success(), "{run_output:?}");
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout).trim(),
            "hni fast:run-script dev"
        );
    });
}

#[test]
fn node_run_falls_back_to_native_when_script_uses_lifecycle_env() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        let fake_node = work.path().join("fake-node");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"node -e \"console.log(process.env.npm_lifecycle_event, process.env.INIT_CWD, process.env.npm_execpath, process.env.npm_node_execpath)\""}} "#.trim(),
        )
        .unwrap();
        fs::write(
            &fake_node,
            "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  printf '  --run\\n'\n  exit 0\nfi\nexit 0\n",
        )
        .unwrap();
        make_executable(&fake_node);

        let run_output = run_hni(
            vec![
                "node",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "--debug-resolved",
                "run",
                "dev",
            ],
            &[
                ("HNI_SKIP_PM_CHECK", "1"),
                ("HNI_REAL_NODE", fake_node.to_str().unwrap()),
            ],
        );
        assert!(run_output.status.success(), "{run_output:?}");
        assert_eq!(
            String::from_utf8_lossy(&run_output.stdout).trim(),
            "hni fast:run-script dev"
        );
    });
}

#[test]
fn node_exec_inherits_native_resolution() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        let bin_dir = project.join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"x"}"#).unwrap();

        let bin = bin_dir.join("hello");
        fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
        make_executable(&bin);

        let exec_output = run_hni(
            vec![
                "node",
                "-C",
                project.to_str().unwrap(),
                "--fast",
                "--debug-resolved",
                "exec",
                "hello",
                "world",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(exec_output.status.success(), "{exec_output:?}");
        assert_eq!(
            String::from_utf8_lossy(&exec_output.stdout).trim(),
            "hni fast:run-local-bin hello world"
        );
    });
}

fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}
