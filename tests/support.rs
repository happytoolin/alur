#![allow(dead_code)]

use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn with_env_lock<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let lock = ENV_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().expect("env lock poisoned");
    f()
}

/// Set a process environment variable for tests.
pub fn set_var<K, V>(key: K, value: V)
where
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    // SAFETY: Callers must hold the global env lock via `with_env_lock`, ensuring
    // serialized environment mutation within the test process.
    unsafe { std::env::set_var(key, value) };
}

/// Remove a process environment variable for tests.
pub fn remove_var<K>(key: K)
where
    K: AsRef<OsStr>,
{
    // SAFETY: Callers must hold the global env lock via `with_env_lock`, ensuring
    // serialized environment mutation within the test process.
    unsafe { std::env::remove_var(key) };
}

pub fn with_var_removed<F, T>(key: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    struct RestoreEnv {
        key: String,
        previous: Option<std::ffi::OsString>,
    }

    impl Drop for RestoreEnv {
        fn drop(&mut self) {
            match self.previous.as_ref() {
                Some(value) => set_var(&self.key, value),
                None => remove_var(&self.key),
            }
        }
    }

    let _restore = RestoreEnv {
        key: key.to_string(),
        previous: std::env::var_os(key),
    };
    remove_var(key);
    f()
}

/// Run hni with the given arguments and extra environment variables.
pub fn run_hni(args: Vec<&str>, extra_env: &[(&str, &str)]) -> std::process::Output {
    let owned_args = args
        .into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    run_hni_owned(&owned_args, extra_env)
}

pub fn run_hni_owned(args: &[String], extra_env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = Command::new(hni_executable_path());
    cmd.args(args)
        .env_remove("HNI_CONFIG_FILE")
        .env_remove("HNI_DEFAULT_PACKAGE_MANAGER")
        .env_remove("HNI_GLOBAL_PACKAGE_MANAGER")
        .env_remove("HNI_FAST")
        .env_remove("HNI_SKIP_PM_CHECK")
        .env_remove("HNI_REAL_NODE")
        .env_remove("HNI_NODE_SHIM_ACTIVE");

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    cmd.output().expect("failed to run hni")
}

pub fn run_command(
    program: &str,
    args: &[String],
    cwd: &Path,
    extra_env: &[(&str, &str)],
) -> std::process::Output {
    let mut cmd = Command::new(program);
    cmd.args(args).current_dir(cwd);

    for (key, value) in extra_env {
        cmd.env(key, value);
    }

    cmd.output()
        .unwrap_or_else(|error| panic!("failed to run {program}: {error}"))
}

pub fn command_exists(program: &str) -> bool {
    which::which(program).is_ok()
}

pub fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

pub fn fixture_path(category: &str, name: &str) -> PathBuf {
    fixture_root().join(category).join(name)
}

pub fn copy_fixture_into(category: &str, name: &str, dest: &Path) {
    copy_dir_all(&fixture_path(category, name), dest)
        .unwrap_or_else(|error| panic!("failed to copy fixture {category}/{name}: {error}"));
}

pub fn real_node_supports_run() -> bool {
    let output = match Command::new("node").arg("--help").output() {
        Ok(output) => output,
        Err(_) => return false,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.contains("--run")
}

/// Get the path to the hni executable.
pub fn hni_executable_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_hni") {
        return PathBuf::from(path);
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push(if cfg!(windows) { "hni.exe" } else { "hni" });
    path
}

fn copy_dir_all(src: &Path, dest: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dest)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dest.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if file_type.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), target)?;
        }
    }

    Ok(())
}
