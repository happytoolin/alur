use std::{
    env,
    ffi::OsString,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow};

use super::paths_equal;

pub const REAL_NODE_ENV: &str = "HNI_REAL_NODE";
pub const SHIM_ACTIVE_ENV: &str = "HNI_NODE_SHIM_ACTIVE";
pub const NODE_SHIM_ENV: &str = "HNI_NODE";

const NODE_RUN_PROBE_TIMEOUT: Duration = Duration::from_secs(1);

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

pub fn real_node_supports_run() -> bool {
    resolve_real_node_path()
        .ok()
        .is_some_and(|path| probe_node_run_support(&path))
}

fn resolve_real_node_path_from_sources() -> Result<Option<PathBuf>> {
    if let Some(recorded) = read_recorded_real_node_path()?
        && recorded.exists()
    {
        return Ok(Some(recorded));
    }

    Ok(scan_path_for_real_node())
}

fn probe_node_run_support(node_path: &Path) -> bool {
    let mut child = match Command::new(node_path)
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };

    let deadline = Instant::now() + NODE_RUN_PROBE_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut pipe) = child.stdout.take() {
        let _ = pipe.read_to_end(&mut stdout);
    }
    if let Some(mut pipe) = child.stderr.take() {
        let _ = pipe.read_to_end(&mut stderr);
    }
    let _ = child.wait();

    help_text_supports_run(&stdout) || help_text_supports_run(&stderr)
}

fn help_text_supports_run(output: &[u8]) -> bool {
    String::from_utf8_lossy(output).contains("--run")
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

    #[test]
    fn help_text_supports_run_detects_run_flag() {
        assert!(help_text_supports_run(
            b"  --run  Run a script from package.json\n"
        ));
        assert!(!help_text_supports_run(b"node help without task runner\n"));
    }

    #[cfg(unix)]
    #[test]
    fn real_node_supports_run_uses_env_override() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let original = env::var_os(REAL_NODE_ENV);
        let dir = tempdir().unwrap();
        let fake_node = dir.path().join("node");
        fs::write(
            &fake_node,
            "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  printf '  --run\\n'\n  exit 0\nfi\nexit 0\n",
        )
        .unwrap();
        let mut perms = fs::metadata(&fake_node).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_node, perms).unwrap();

        unsafe { env::set_var(REAL_NODE_ENV, &fake_node) };
        assert!(real_node_supports_run());

        match original {
            Some(value) => unsafe { env::set_var(REAL_NODE_ENV, value) },
            None => unsafe { env::remove_var(REAL_NODE_ENV) },
        }
    }

    #[cfg(unix)]
    #[test]
    fn real_node_supports_run_times_out_for_hanging_help() {
        use std::os::unix::fs::PermissionsExt;
        use std::time::{Duration, Instant};

        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let original = env::var_os(REAL_NODE_ENV);
        let dir = tempdir().unwrap();
        let fake_node = dir.path().join("node");
        fs::write(
            &fake_node,
            "#!/bin/sh\nif [ \"$1\" = \"--help\" ]; then\n  sleep 5\n  exit 0\nfi\nexit 0\n",
        )
        .unwrap();
        let mut perms = fs::metadata(&fake_node).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_node, perms).unwrap();

        unsafe { env::set_var(REAL_NODE_ENV, &fake_node) };
        let start = Instant::now();
        assert!(!real_node_supports_run());
        assert!(start.elapsed() < Duration::from_secs(2));

        match original {
            Some(value) => unsafe { env::set_var(REAL_NODE_ENV, value) },
            None => unsafe { env::remove_var(REAL_NODE_ENV) },
        }
    }
}
