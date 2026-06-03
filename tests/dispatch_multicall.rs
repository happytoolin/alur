use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

mod support;

#[test]
fn multicall_aliases_resolve_expected_commands() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let fixtures = work.path().join("fixtures");
        fs::create_dir_all(&fixtures).unwrap();

        let npm_proj = fixtures.join("npm");
        fs::create_dir_all(&npm_proj).unwrap();
        fs::write(
            npm_proj.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();
        fs::write(npm_proj.join("package-lock.json"), "lock").unwrap();

        let bin_dir = work.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let exe = hni_executable_path();
        if !exe.exists() {
            return;
        }
        create_alias(&exe, &bin_dir, "ni");
        create_alias(&exe, &bin_dir, "nr");
        create_alias(&exe, &bin_dir, "nlx");
        create_alias(&exe, &bin_dir, "nru");
        create_alias(&exe, &bin_dir, "nun");
        create_alias(&exe, &bin_dir, "nci");
        create_alias(&exe, &bin_dir, "np");
        create_alias(&exe, &bin_dir, "ns");
        create_alias(&exe, &bin_dir, "node");

        let ni_out = run_alias(
            &bin_dir,
            "ni",
            vec!["-C", npm_proj.to_str().unwrap(), "vite", "--print-command"],
            &[],
        );
        assert_eq!(ni_out.trim(), "npm i vite");

        let nr_out = run_alias(
            &bin_dir,
            "nr",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "--pm",
                "dev",
                "--port=3000",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(nr_out.trim(), "npm run dev -- --port=3000");

        let nr_if_present_out = run_alias(
            &bin_dir,
            "nr",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "--pm",
                "--if-present",
                "missing-script",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(
            nr_if_present_out.trim(),
            "npm run --if-present missing-script"
        );

        let ni_frozen_if_present_out = run_alias(
            &bin_dir,
            "ni",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "--frozen-if-present",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(ni_frozen_if_present_out.trim(), "npm ci");

        let ni_global_out = run_alias(
            &bin_dir,
            "ni",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "-g",
                "eslint",
                "--print-command",
            ],
            &[("HNI_GLOBAL_PACKAGE_MANAGER", "yarn")],
        );
        assert_eq!(ni_global_out.trim(), "yarn global add eslint");

        let node_out = run_alias(
            &bin_dir,
            "node",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "--pm",
                "run",
                "dev",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(node_out.trim(), "npm run dev");

        let nlx_out = run_alias(
            &bin_dir,
            "nlx",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "--pm",
                "--print-command",
                "vitest",
                "--",
                "--help",
            ],
            &[],
        );
        assert!(
            nlx_out.trim().contains("npx vitest -- --help"),
            "unexpected nlx print-command output: {}",
            nlx_out.trim()
        );

        let nru_out = run_alias(
            &bin_dir,
            "nru",
            vec!["-C", npm_proj.to_str().unwrap(), "vite", "--print-command"],
            &[],
        );
        assert_eq!(nru_out.trim(), "npm update vite");

        let nci_out = run_alias(
            &bin_dir,
            "nci",
            vec!["-C", npm_proj.to_str().unwrap(), "--print-command"],
            &[],
        );
        assert_eq!(nci_out.trim(), "npm ci");

        let nun_global_out = run_alias(
            &bin_dir,
            "nun",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "-g",
                "eslint",
                "--print-command",
            ],
            &[("HNI_GLOBAL_PACKAGE_MANAGER", "yarn")],
        );
        assert_eq!(nun_global_out.trim(), "yarn global remove eslint");

        let np_out = run_alias(
            &bin_dir,
            "np",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "echo one",
                "echo two",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(
            np_out.trim(),
            "hni batch:parallel \"echo one\" \"echo two\""
        );

        let ns_out = run_alias(
            &bin_dir,
            "ns",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "echo one",
                "echo two",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(
            ns_out.trim(),
            "hni batch:sequential \"echo one\" \"echo two\""
        );

        let node_parallel_out = run_alias(
            &bin_dir,
            "node",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "p",
                "echo one",
                "echo two",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(
            node_parallel_out.trim(),
            "hni batch:parallel \"echo one\" \"echo two\""
        );

        let node_sequential_out = run_alias(
            &bin_dir,
            "node",
            vec![
                "-C",
                npm_proj.to_str().unwrap(),
                "s",
                "echo one",
                "echo two",
                "--print-command",
            ],
            &[],
        );
        assert_eq!(
            node_sequential_out.trim(),
            "hni batch:sequential \"echo one\" \"echo two\""
        );

        let fake_node = work.path().join(if cfg!(windows) {
            "real-node.exe"
        } else {
            "real-node"
        });
        fs::write(&fake_node, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_node).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_node, perms).unwrap();
        }

        let passthrough_out = run_alias(
            &bin_dir,
            "node",
            vec!["script.js", "--print-command"],
            &[("HNI_REAL_NODE", fake_node.to_str().unwrap())],
        );
        let output = passthrough_out.trim();
        assert!(output.contains(fake_node.to_string_lossy().as_ref()));
        assert!(output.contains("script.js"));

        let node_flag_out = run_alias(
            &bin_dir,
            "node",
            vec!["-p", "1+1", "--print-command"],
            &[("HNI_REAL_NODE", fake_node.to_str().unwrap())],
        );
        let output = node_flag_out.trim();
        assert!(output.contains(fake_node.to_string_lossy().as_ref()));
        assert!(output.contains("-p"));
        assert!(output.contains("1+1"));
    });
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

fn run_alias(bin_dir: &Path, alias: &str, args: Vec<&str>, extra_env: &[(&str, &str)]) -> String {
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

    let output = cmd.output().expect("failed to run alias binary");
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).to_string()
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
