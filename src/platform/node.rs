use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};

use super::paths_equal;

pub const REAL_NODE_ENV: &str = "HNI_REAL_NODE";
pub const SHIM_ACTIVE_ENV: &str = "HNI_NODE_SHIM_ACTIVE";
pub const NODE_SHIM_ENV: &str = "HNI_NODE";

pub fn resolve_real_node_path() -> Result<PathBuf> {
    if let Some(from_env) = env::var_os(REAL_NODE_ENV) {
        let path = PathBuf::from(from_env);
        if path.exists() {
            return Ok(path);
        }

        return Err(anyhow!(
            "{} points to a missing path: {}",
            REAL_NODE_ENV,
            path.display()
        ));
    }

    resolve_real_node_path_from_sources()?.ok_or_else(|| {
        anyhow!(
            "unable to locate real node binary. Set {}=/absolute/path/to/node",
            REAL_NODE_ENV
        )
    })
}

fn resolve_real_node_path_from_sources() -> Result<Option<PathBuf>> {
    if let Some(recorded) = read_recorded_real_node_path()?
        && recorded.exists()
    {
        return Ok(Some(recorded));
    }

    Ok(scan_path_for_real_node())
}
pub fn recorded_real_node_path_file() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("hni").join("real-node-path"))
}

fn read_recorded_real_node_path() -> Result<Option<PathBuf>> {
    let Some(path) = recorded_real_node_path_file() else {
        return Ok(None);
    };

    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    Ok(Some(PathBuf::from(trimmed)))
}

fn scan_path_for_real_node() -> Option<PathBuf> {
    let current_exe = env::current_exe().ok();
    let current_dir = current_exe
        .as_ref()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    let candidates = which::which_all("node").ok()?;
    for candidate in candidates {
        if should_skip_node_candidate(&candidate, current_exe.as_deref(), current_dir.as_deref()) {
            continue;
        }
        return Some(candidate);
    }

    None
}

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
    current_dir: Option<&Path>,
) -> bool {
    if let Some(current_dir) = current_dir
        && let Some(parent) = candidate.parent()
        && paths_equal(parent, current_dir)
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
        Some("hni") | Some("hni.exe")
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
    fn skips_node_candidates_that_resolve_to_hni() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let release_dir = dir.path().join("release");
        let debug_dir = dir.path().join("debug");
        let shim_dir = dir.path().join("shim");

        fs::create_dir_all(&release_dir).unwrap();
        fs::create_dir_all(&debug_dir).unwrap();
        fs::create_dir_all(&shim_dir).unwrap();

        let release_hni = release_dir.join("hni");
        let debug_hni = debug_dir.join("hni");
        fs::write(&release_hni, b"release").unwrap();
        fs::write(&debug_hni, b"debug").unwrap();
        symlink(&release_hni, shim_dir.join("node")).unwrap();

        assert!(should_skip_node_candidate(
            &shim_dir.join("node"),
            Some(&debug_hni),
            Some(&debug_dir),
        ));
    }

    #[test]
    fn env_override_takes_effect() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let original = env::var_os(REAL_NODE_ENV);
        let dir = tempdir().unwrap();
        let fake_node = dir.path().join("node");
        fs::write(&fake_node, b"node").unwrap();

        unsafe { env::set_var(REAL_NODE_ENV, &fake_node) };
        assert_eq!(resolve_real_node_path().unwrap(), fake_node);

        match original {
            Some(value) => unsafe { env::set_var(REAL_NODE_ENV, value) },
            None => unsafe { env::remove_var(REAL_NODE_ENV) },
        }
    }
}
