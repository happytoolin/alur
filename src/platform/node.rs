use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use is_executable::IsExecutable;

use super::paths_equal;

pub const REAL_NODE_ENV: &str = "ALUR_REAL_NODE";

#[must_use]
pub fn node_binary_name() -> &'static str {
    if cfg!(windows) { "node.exe" } else { "node" }
}

#[must_use]
pub fn managed_node_shim_dir() -> Option<PathBuf> {
    local_data_dir()
        .or_else(config_dir)
        .map(|d| d.join("alur").join("bin"))
}

fn local_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(path) = env_path("LOCALAPPDATA") {
        return Some(path);
    }

    dirs::data_local_dir()
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    if let Some(path) = env_path("APPDATA") {
        return Some(path);
    }

    dirs::config_dir()
}

#[cfg(windows)]
fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[must_use]
pub fn managed_node_shim_path() -> Option<PathBuf> {
    managed_node_shim_dir().map(|dir| dir.join(node_binary_name()))
}

/// Resolves the real Node.js binary that alur should delegate to.
///
/// # Errors
///
/// Returns an error when `ALUR_REAL_NODE` points to a missing path or no non-alur Node.js binary can
/// be found on `PATH`.
pub fn resolve_real_node_path() -> Result<PathBuf> {
    if let Some(from_env) = env::var_os(REAL_NODE_ENV) {
        let path = PathBuf::from(from_env);
        if path.exists() {
            return Ok(path);
        }

        return Err(anyhow!(
            "{REAL_NODE_ENV} points to a missing path: {}",
            path.display()
        ));
    }

    resolve_real_node_path_from_sources().ok_or_else(|| {
        anyhow!("unable to locate real node binary. Set {REAL_NODE_ENV}=/absolute/path/to/node")
    })
}

fn resolve_real_node_path_from_sources() -> Option<PathBuf> {
    scan_path_for_real_node()
}

fn scan_path_for_real_node() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok();
    let managed_shim_dir = managed_node_shim_dir();
    let path_var = env::var_os("PATH")?;

    for mut candidate in env::split_paths(&path_var) {
        candidate.push(node_binary_name());
        if !candidate.is_executable() {
            continue;
        }

        if should_skip_node_candidate(
            &candidate,
            current_exe.as_deref(),
            managed_shim_dir.as_deref(),
        ) {
            continue;
        }
        return Some(candidate);
    }

    None
}

#[must_use]
pub fn path_with_real_node_priority(
    real_node: &Path,
    current_path: Option<OsString>,
) -> Option<OsString> {
    let real_node_dir = real_node.parent()?;
    let canonical_real_node_dir = dunce::canonicalize(real_node_dir).ok();
    let mut ordered = Vec::new();
    ordered.push(real_node_dir.to_path_buf());

    if let Some(current_path) = current_path {
        ordered.extend(env::split_paths(&current_path).filter(|entry| {
            !path_matches_real_node_dir(entry, real_node_dir, canonical_real_node_dir.as_deref())
        }));
    }

    env::join_paths(ordered).ok()
}

fn should_skip_node_candidate(
    candidate: &Path,
    current_exe: Option<&Path>,
    managed_shim_dir: Option<&Path>,
) -> bool {
    if let Some(managed_shim_dir) = managed_shim_dir
        && let Some(parent) = candidate.parent()
        && paths_equal(parent, managed_shim_dir)
    {
        return true;
    }

    if let Some(current_exe) = current_exe
        && paths_equal(candidate, current_exe)
    {
        return true;
    }

    matches!(
        dunce::canonicalize(candidate)
            .ok()
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        Some("alur" | "alur.exe")
    )
}

fn path_matches_real_node_dir(
    candidate: &Path,
    real_node_dir: &Path,
    canonical_real_node_dir: Option<&Path>,
) -> bool {
    candidate == real_node_dir
        || canonical_real_node_dir
            .and_then(|canonical_real_node_dir| {
                dunce::canonicalize(candidate)
                    .ok()
                    .map(|path| path == canonical_real_node_dir)
            })
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::Mutex};
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn path_with_real_node_priority_prepends_real_node_dir_once() {
        let current_path = env::join_paths([
            PathBuf::from("shim"),
            PathBuf::from("real"),
            PathBuf::from("other"),
        ])
        .unwrap();
        let path =
            path_with_real_node_priority(Path::new("real/node"), Some(current_path)).unwrap();
        let entries = env::split_paths(&path).collect::<Vec<_>>();

        assert_eq!(
            entries,
            vec![
                PathBuf::from("real"),
                PathBuf::from("shim"),
                PathBuf::from("other"),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn skips_node_candidates_that_resolve_to_alur() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let release_dir = dir.path().join("release");
        let debug_dir = dir.path().join("debug");
        let shim_dir = dir.path().join("shim");

        fs::create_dir_all(&release_dir).unwrap();
        fs::create_dir_all(&debug_dir).unwrap();
        fs::create_dir_all(&shim_dir).unwrap();

        let release_alur = release_dir.join("alur");
        let debug_alur = debug_dir.join("alur");
        fs::write(&release_alur, b"release").unwrap();
        fs::write(&debug_alur, b"debug").unwrap();
        symlink(&release_alur, shim_dir.join("node")).unwrap();

        assert!(should_skip_node_candidate(
            &shim_dir.join("node"),
            Some(&debug_alur),
            None,
        ));
    }

    #[test]
    fn keeps_real_node_candidates_in_current_alur_dir() {
        let dir = tempdir().unwrap();
        let bin_dir = dir.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        let current_alur = bin_dir.join("alur");
        let real_node = bin_dir.join("node");
        fs::write(&current_alur, b"alur").unwrap();
        fs::write(&real_node, b"node").unwrap();

        assert!(!should_skip_node_candidate(
            &real_node,
            Some(&current_alur),
            None,
        ));
    }

    #[test]
    fn env_override_takes_effect() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let original = env::var_os(REAL_NODE_ENV);
        let dir = tempdir().unwrap();
        let fake_node = dir.path().join("node");
        fs::write(&fake_node, b"node").unwrap();

        // SAFETY: ENV_LOCK serializes this test's process-wide environment mutation.
        unsafe { env::set_var(REAL_NODE_ENV, &fake_node) };
        assert_eq!(resolve_real_node_path().unwrap(), fake_node);

        match original {
            // SAFETY: ENV_LOCK is still held while restoring the environment.
            Some(value) => unsafe { env::set_var(REAL_NODE_ENV, value) },
            // SAFETY: ENV_LOCK is still held while restoring the environment.
            None => unsafe { env::remove_var(REAL_NODE_ENV) },
        }
    }
}
