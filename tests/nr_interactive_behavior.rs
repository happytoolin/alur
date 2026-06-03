use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

mod support;

#[test]
fn completion_query_filters_scripts_by_prefix() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = prepare_script_project(work.path());
        let bin_dir = prepare_nr_alias_dir(work.path());

        let out = run_nr(
            &bin_dir,
            vec!["-C", project.to_str().unwrap(), "--completion", "d"],
            &[("COMP_CWORD", "2")],
        );
        assert!(out.status.success());

        let stdout = String::from_utf8_lossy(&out.stdout);
        let lines = stdout.lines().collect::<Vec<_>>();
        assert_eq!(lines, vec!["dev"]);
    });
}

#[test]
fn completion_query_uses_second_completion_arg_for_early_comp_word() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = prepare_script_project(work.path());
        let bin_dir = prepare_nr_alias_dir(work.path());

        let out = run_nr(
            &bin_dir,
            vec![
                "-C",
                project.to_str().unwrap(),
                "--completion",
                "ignored",
                "b",
            ],
            &[("COMP_CWORD", "1")],
        );
        assert!(out.status.success());

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(stdout.lines().collect::<Vec<_>>(), vec!["build"]);
    });
}

#[test]
fn completion_query_without_prefix_lists_sorted_scripts() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = prepare_script_project(work.path());
        let bin_dir = prepare_nr_alias_dir(work.path());

        let out = run_nr(
            &bin_dir,
            vec!["-C", project.to_str().unwrap(), "--completion"],
            &[],
        );
        assert!(out.status.success());

        let stdout = String::from_utf8_lossy(&out.stdout);
        assert_eq!(
            stdout.lines().collect::<Vec<_>>(),
            vec!["build", "dev", "test"]
        );
    });
}

fn prepare_script_project(workdir: &Path) -> PathBuf {
    let project = workdir.join("proj");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("package.json"),
        r#"{"name":"x","scripts":{"dev":"vite","build":"vite build","test":"vitest"}}"#,
    )
    .unwrap();
    project
}

fn prepare_nr_alias_dir(workdir: &Path) -> PathBuf {
    let dir = workdir.join("bin");
    fs::create_dir_all(&dir).unwrap();

    let exe = hni_executable_path();
    if !exe.exists() {
        panic!("hni executable not found at {}", exe.display());
    }

    create_alias(&exe, &dir, "nr");
    dir
}

fn run_nr(bin_dir: &Path, args: Vec<&str>, extra_env: &[(&str, &str)]) -> Output {
    let alias = if cfg!(windows) { "nr.exe" } else { "nr" };

    let mut cmd = Command::new(bin_dir.join(alias));
    cmd.args(args).env("HNI_SKIP_PM_CHECK", "1");

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    cmd.output().expect("failed to run nr alias")
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
