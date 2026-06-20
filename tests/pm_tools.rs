use std::{env, fs};

mod support;

#[test]
fn pm_which_prints_detected_binary_and_source() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        fs::write(
            work.path().join("package.json"),
            r#"{"packageManager":"npm@10.0.0"}"#,
        )
        .unwrap();

        let output = support::run_alur(
            vec!["pm", "-C", work.path().to_str().unwrap(), "which"],
            &[("ALUR_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert!(!String::from_utf8_lossy(&output.stdout).trim().is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("PackageManagerField"));
    });
}

#[test]
fn pm_use_updates_package_manager_field() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let package_json = work.path().join("package.json");
        fs::write(&package_json, r#"{"name":"demo"}"#).unwrap();

        let output = support::run_alur(
            vec![
                "pm",
                "-C",
                work.path().to_str().unwrap(),
                "use",
                "pnpm@9.15.4",
            ],
            &[],
        );

        assert!(output.status.success(), "{output:?}");
        let raw = fs::read_to_string(package_json).unwrap();
        assert!(raw.contains(r#""packageManager": "pnpm@9.15.4""#));
    });
}

#[test]
fn pm_shim_creates_managed_wrappers() {
    support::with_env_lock(|| {
        let shim_root = tempfile::tempdir().unwrap();
        let shim_dir = shim_root.path().join("pm-shims");

        let output = support::run_alur(
            vec!["pm", "shim"],
            &[("ALUR_PM_SHIM_DIR", shim_dir.to_str().unwrap())],
        );

        assert!(output.status.success(), "{output:?}");
        assert!(
            shim_dir
                .join(if cfg!(windows) { "npm.cmd" } else { "npm" })
                .is_file()
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains(shim_dir.to_str().unwrap()),
            "{output:?}"
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(shim_dir.join("npm"))
                .unwrap()
                .permissions()
                .mode();
            assert_ne!(mode & 0o111, 0);
        }
    });
}

#[cfg(unix)]
#[test]
fn pm_run_routes_to_detected_package_manager_without_recursing_into_shims() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("project");
        let fake_bin = work.path().join("fake-bin");
        let fake_shims = work.path().join("fake-shims");
        let marker = work.path().join("args.txt");
        fs::create_dir_all(&project).unwrap();
        fs::create_dir_all(&fake_bin).unwrap();
        fs::create_dir_all(&fake_shims).unwrap();
        fs::write(
            project.join("package.json"),
            r#"{"packageManager":"pnpm@9.0.0"}"#,
        )
        .unwrap();

        let pnpm = fake_bin.join("pnpm");
        fs::write(
            &pnpm,
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$ALUR_MARKER\"\n",
        )
        .unwrap();
        make_executable(&pnpm);

        let path = env::join_paths([fake_shims.as_path(), fake_bin.as_path()]).unwrap();
        let output = support::run_alur(
            vec![
                "pm",
                "-C",
                project.to_str().unwrap(),
                "run",
                "npx",
                "eslint",
                "--fix",
            ],
            &[
                ("ALUR_PM_SHIM_DIR", fake_shims.to_str().unwrap()),
                ("ALUR_MARKER", marker.to_str().unwrap()),
                ("PATH", path.to_str().unwrap()),
            ],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(fs::read_to_string(marker).unwrap(), "dlx\neslint\n--fix\n");
    });
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}
