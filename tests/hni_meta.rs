use std::{fs, path::PathBuf, process::Command};

mod support;

#[test]
fn hni_subcommand_aliases_resolve_like_multicall() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"x"}"#).unwrap();

        let output = run_hni(
            vec![
                "ni",
                "-C",
                project.to_str().unwrap(),
                "vite",
                "--dry-run",
                "--explain",
            ],
            &[],
        );
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("hni explain"));
        assert!(stdout.contains("resolved:"));
        assert!(stdout.contains("npm i vite"));
    });
}

#[test]
fn hni_doctor_completion_and_init_are_available() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let doctor = run_hni(vec!["doctor"], &[("HNI_SKIP_PM_CHECK", "1")]);
        assert!(doctor.status.success());
        let doctor_out = String::from_utf8_lossy(&doctor.stdout);
        assert!(doctor_out.contains("hni doctor"));

        let completion = run_hni(vec!["completion", "bash"], &[]);
        assert!(completion.status.success());
        let completion_out = String::from_utf8_lossy(&completion.stdout);
        assert!(completion_out.contains("hni"));

        let init_home = work.path().join("init-home");
        let init_data = work.path().join("init-data");
        fs::create_dir_all(&init_home).unwrap();
        fs::create_dir_all(&init_data).unwrap();
        let init = run_hni(
            vec!["init", "bash"],
            &[
                ("HOME", init_home.to_string_lossy().as_ref()),
                ("XDG_DATA_HOME", init_data.to_string_lossy().as_ref()),
            ],
        );
        assert!(init.status.success());
        let init_out = String::from_utf8_lossy(&init.stdout);
        assert!(init_out.contains("# hni init"));
    });
}

fn run_hni(args: Vec<&str>, extra_env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(hni_executable_path());
    cmd.args(args).env("HNI_SKIP_PM_CHECK", "1");

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    cmd.output().expect("failed to run hni")
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
