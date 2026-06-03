use std::path::{Component, Path, PathBuf};

use crate::core::util::file_is_runnable;

pub fn resolve_local_bin(bin_name: &str, bin_dirs: &[PathBuf]) -> Option<PathBuf> {
    crate::core::profile::measure("local_bin.lookup", || {
        if !is_safe_bin_name(bin_name) {
            return None;
        }

        #[cfg(windows)]
        const SUFFIXES: &[&str] = &["", ".cmd", ".exe", ".bat", ".ps1"];
        #[cfg(not(windows))]
        const SUFFIXES: &[&str] = &[""];

        for dir in bin_dirs {
            for suffix in SUFFIXES {
                let candidate = dir.join(format!("{bin_name}{suffix}"));
                if file_is_runnable(&candidate) {
                    return Some(candidate);
                }
            }
        }

        None
    })
}

fn is_safe_bin_name(bin_name: &str) -> bool {
    let mut components = Path::new(bin_name).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_path_like_local_bin_names() {
        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(dir.path().join("escape"), "").unwrap();

        assert_eq!(resolve_local_bin("../escape", &[bin_dir]), None);
    }

    #[test]
    fn does_not_return_directories_from_local_bin_lookup() {
        let dir = tempfile::tempdir().unwrap();
        let bin_dir = dir.path().join("node_modules").join(".bin");
        fs::create_dir_all(bin_dir.join("tool")).unwrap();

        assert_eq!(resolve_local_bin("tool", &[bin_dir]), None);
    }
}
