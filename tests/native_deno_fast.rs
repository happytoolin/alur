#![cfg(unix)]

use std::fs;

mod support;

use support::run_hni;

#[test]
fn deno_fast_path_uses_native_task_execution() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{"tasks":{"dev":"echo ok > result.txt"}}"#,
        )
        .unwrap();

        support::with_var_removed("HNI_NATIVE", || {
            let output = run_hni(
                vec![
                    "nr",
                    "-C",
                    work.path().to_str().unwrap(),
                    "--debug-resolved",
                    "dev",
                ],
                &[("HNI_SKIP_PM_CHECK", "1")],
            );

            assert!(output.status.success(), "{output:?}");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout).trim(),
                "hni native:run-deno-task dev"
            );
        });
    });
}

#[test]
fn deno_no_native_still_delegates() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{"tasks":{"dev":"echo ok > result.txt"}}"#,
        )
        .unwrap();

        let output = run_hni(
            vec![
                "nr",
                "-C",
                work.path().to_str().unwrap(),
                "--no-native",
                "--debug-resolved",
                "dev",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "deno task dev"
        );
    });
}

#[test]
fn deno_jsonc_is_supported() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.jsonc"),
            r#"{
              // comment
              "tasks": {
                "dev": "echo ok > result.txt"
              }
            }"#,
        )
        .unwrap();

        let output = run_hni(
            vec![
                "nr",
                "-C",
                work.path().to_str().unwrap(),
                "--native",
                "--debug-resolved",
                "dev",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "hni native:run-deno-task dev"
        );
    });
}

#[test]
fn deno_task_cwd_and_init_cwd_match_deno_behavior() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().join("project");
        let nested = root.join("nested").join("child");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            root.join("deno.json"),
            r#"{"tasks":{"dev":"echo $PWD > pwd.txt && echo $INIT_CWD > init.txt"}}"#,
        )
        .unwrap();

        let output = run_hni(
            vec!["nr", "-C", nested.to_str().unwrap(), "--native", "dev"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            fs::read_to_string(root.join("pwd.txt")).unwrap().trim(),
            root.to_str().unwrap()
        );
        assert_eq!(
            fs::read_to_string(root.join("init.txt")).unwrap().trim(),
            nested.to_str().unwrap()
        );
    });
}

#[test]
fn deno_dependencies_run_before_root() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{
              "tasks": {
                "dep-a": "echo dep-a >> order.txt",
                "dep-b": "echo dep-b >> order.txt",
                "dev": {
                  "dependencies": ["dep-a", "dep-b"],
                  "command": "echo root >> order.txt"
                }
              }
            }"#,
        )
        .unwrap();

        let output = run_hni(
            vec!["nr", "-C", work.path().to_str().unwrap(), "--native", "dev"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        let lines = fs::read_to_string(work.path().join("order.txt"))
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2], "root");
        assert!(lines[..2].contains(&"dep-a".to_string()));
        assert!(lines[..2].contains(&"dep-b".to_string()));
    });
}

#[test]
fn deno_dependency_only_task_is_valid() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{
              "tasks": {
                "dep-a": "echo dep-a >> order.txt",
                "dep-b": "echo dep-b >> order.txt",
                "dev": {
                  "dependencies": ["dep-a", "dep-b"]
                }
              }
            }"#,
        )
        .unwrap();

        let output = run_hni(
            vec!["nr", "-C", work.path().to_str().unwrap(), "--native", "dev"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        let lines = fs::read_to_string(work.path().join("order.txt"))
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines.contains(&"dep-a".to_string()));
        assert!(lines.contains(&"dep-b".to_string()));
    });
}

#[test]
fn deno_wildcard_task_selection_runs_matching_tasks() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{
              "tasks": {
                "build-a": "echo a > a.txt",
                "build-b": "echo b > b.txt",
                "test": "echo test > test.txt"
              }
            }"#,
        )
        .unwrap();

        let output = run_hni(
            vec![
                "nr",
                "-C",
                work.path().to_str().unwrap(),
                "--native",
                "build-*",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            fs::read_to_string(work.path().join("a.txt"))
                .unwrap()
                .trim(),
            "a"
        );
        assert_eq!(
            fs::read_to_string(work.path().join("b.txt"))
                .unwrap()
                .trim(),
            "b"
        );
        assert!(!work.path().join("test.txt").exists());
    });
}

#[test]
fn deno_mixed_project_prefers_deno_task_over_package_json_script() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{"tasks":{"dev":"echo deno > winner.txt"}}"#,
        )
        .unwrap();
        fs::write(
            work.path().join("package.json"),
            r#"{"name":"x","scripts":{"dev":"echo package > winner.txt"}}"#,
        )
        .unwrap();

        let output = run_hni(
            vec!["nr", "-C", work.path().to_str().unwrap(), "--native", "dev"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            fs::read_to_string(work.path().join("winner.txt"))
                .unwrap()
                .trim(),
            "deno"
        );
    });
}

#[test]
fn deno_package_json_fallback_runs_pre_and_post() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{"tasks":{"build":"echo build > build.txt"}}"#,
        )
        .unwrap();
        fs::write(
            work.path().join("package.json"),
            r#"{"name":"x","scripts":{"predev":"echo pre >> order.txt","dev":"echo main >> order.txt","postdev":"echo post >> order.txt"}}"#,
        )
        .unwrap();

        let output = run_hni(
            vec!["nr", "-C", work.path().to_str().unwrap(), "--native", "dev"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        let lines = fs::read_to_string(work.path().join("order.txt"))
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert_eq!(lines, vec!["pre", "main", "post"]);
    });
}

#[test]
fn deno_cycle_and_workspace_fall_back_to_delegated() {
    support::with_env_lock(|| {
        let cycle = tempfile::tempdir().unwrap();
        fs::write(
            cycle.path().join("deno.json"),
            r#"{
              "tasks": {
                "a": {"dependencies":["b"], "command":"echo a"},
                "b": {"dependencies":["a"], "command":"echo b"}
              }
            }"#,
        )
        .unwrap();

        let cycle_out = run_hni(
            vec![
                "nr",
                "-C",
                cycle.path().to_str().unwrap(),
                "--native",
                "--debug-resolved",
                "a",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(cycle_out.status.success(), "{cycle_out:?}");
        assert_eq!(
            String::from_utf8_lossy(&cycle_out.stdout).trim(),
            "deno task a"
        );

        let workspace = tempfile::tempdir().unwrap();
        fs::write(
            workspace.path().join("deno.json"),
            r#"{
              "workspace": ["packages/*"],
              "tasks": {"dev":"echo dev"}
            }"#,
        )
        .unwrap();

        let workspace_out = run_hni(
            vec![
                "nr",
                "-C",
                workspace.path().to_str().unwrap(),
                "--native",
                "--debug-resolved",
                "dev",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(workspace_out.status.success(), "{workspace_out:?}");
        assert_eq!(
            String::from_utf8_lossy(&workspace_out.stdout).trim(),
            "deno task dev"
        );
    });
}

#[test]
fn deno_nlx_native_handles_local_bins_and_delegates_remote() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let bin_dir = work.path().join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(
            work.path().join("deno.json"),
            r#"{"tasks":{"dev":"echo ok"}}"#,
        )
        .unwrap();
        fs::write(bin_dir.join("hello"), "#!/bin/sh\nexit 0\n").unwrap();
        make_executable(&bin_dir.join("hello"));

        let local = run_hni(
            vec![
                "nlx",
                "-C",
                work.path().to_str().unwrap(),
                "--native",
                "--debug-resolved",
                "hello",
                "--flag",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(local.status.success(), "{local:?}");
        assert_eq!(
            String::from_utf8_lossy(&local.stdout).trim(),
            "hni native:run-local-bin hello --flag"
        );

        let remote = run_hni(
            vec![
                "nlx",
                "-C",
                work.path().to_str().unwrap(),
                "--native",
                "--debug-resolved",
                "create-vite",
            ],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(remote.status.success(), "{remote:?}");
        assert_eq!(
            String::from_utf8_lossy(&remote.stdout).trim(),
            "deno run npm:create-vite"
        );
    });
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}
