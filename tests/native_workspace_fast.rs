#![cfg(unix)]

use std::fs;

mod support;

use support::run_alur;

#[test]
fn workspace_recursive_run_uses_fast_topological_order() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let root = work.path();
        let core = root.join("packages/core");
        let app = root.join("packages/app");
        fs::create_dir_all(&core).unwrap();
        fs::create_dir_all(&app).unwrap();
        fs::write(
            root.join("package.json"),
            r#"{"name":"root","packageManager":"npm@10.0.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        fs::write(root.join("package-lock.json"), "lock").unwrap();
        fs::write(
            core.join("package.json"),
            r#"{"name":"core","scripts":{"build":"printf core >> ../../order.txt"}}"#,
        )
        .unwrap();
        fs::write(
            app.join("package.json"),
            r#"{"name":"app","dependencies":{"core":"workspace:*"},"scripts":{"build":"printf app >> ../../order.txt"}}"#,
        )
        .unwrap();

        let output = run_alur(
            vec!["run", "-C", root.to_str().unwrap(), "--fast", "-r", "build"],
            &[("ALUR_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            fs::read_to_string(root.join("order.txt")).unwrap(),
            "coreapp"
        );
    });
}

#[test]
fn workspace_recursive_exec_runs_local_bin_in_each_package() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let root = work.path();
        let core = root.join("packages/core");
        let app = root.join("packages/app");
        let bin_dir = root.join("node_modules/.bin");
        fs::create_dir_all(&core).unwrap();
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(
            root.join("package.json"),
            r#"{"name":"root","packageManager":"npm@10.0.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        fs::write(root.join("package-lock.json"), "lock").unwrap();
        fs::write(core.join("package.json"), r#"{"name":"core"}"#).unwrap();
        fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();

        let bin = bin_dir.join("hello");
        fs::write(
            &bin,
            "#!/bin/sh\nprintf '%s:%s\\n' \"$(basename \"$PWD\")\" \"$1\" >> \"$MARKER\"\n",
        )
        .unwrap();
        make_executable(&bin);
        let marker = root.join("bins.txt");

        let output = run_alur(
            vec![
                "exec",
                "-C",
                root.to_str().unwrap(),
                "--fast",
                "-r",
                "hello",
                "world",
            ],
            &[
                ("ALUR_SKIP_PM_CHECK", "1"),
                ("MARKER", marker.to_str().unwrap()),
            ],
        );

        assert!(output.status.success(), "{output:?}");
        let mut lines = fs::read_to_string(marker)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect::<Vec<_>>();
        lines.sort();
        assert_eq!(lines, vec!["app:world", "core:world"]);
    });
}

fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}
