use std::path::{Component, Path, PathBuf};

use anyhow::Result;

use super::{config::HniConfig, pkg_json::PackageJson, resolve::ProjectState};

#[derive(Debug, Clone)]
pub struct NearestPackage {
    pub root: PathBuf,
    pub package_json_path: PathBuf,
    pub manifest: PackageJson,
}

pub fn find_nearest_package(cwd: &Path) -> Result<Option<NearestPackage>> {
    Ok(ProjectState::scan(cwd, &HniConfig::default())?.nearest_package())
}

pub fn node_modules_bin_dirs(cwd: &Path) -> Vec<PathBuf> {
    crate::core::profile::measure("local_bin.scan_dirs", || {
        let mut bin_dirs = Vec::new();

        for dir in cwd.ancestors() {
            for candidate in [
                dir.join("node_modules").join(".bin"),
                dir.join("node_modules")
                    .join(".pnpm")
                    .join("node_modules")
                    .join(".bin"),
            ] {
                if candidate.is_dir() {
                    bin_dirs.push(candidate);
                }
            }
        }

        bin_dirs
    })
}

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
                if is_runnable_file(&candidate) {
                    return Some(candidate);
                }
            }
        }

        None
    })
}

pub fn resolve_declared_package_bin(cwd: &Path, bin_name: &str) -> Result<Option<PathBuf>> {
    ProjectState::scan(cwd, &HniConfig::default())?.resolve_declared_package_bin(bin_name)
}

fn is_safe_bin_name(bin_name: &str) -> bool {
    let mut components = Path::new(bin_name).components();
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
}

#[cfg(unix)]
fn is_runnable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_runnable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn finds_nearest_package_in_ancestors() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let nested = root.join("packages").join("app");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("package.json"), r#"{"name":"root"}"#).unwrap();

        let found = find_nearest_package(&nested).unwrap().unwrap();
        assert_eq!(found.root, root);
    }

    #[test]
    fn bin_dirs_are_nearest_first() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let nested = root.join("packages").join("app");
        fs::create_dir_all(root.join("node_modules").join(".bin")).unwrap();
        fs::create_dir_all(nested.join("node_modules").join(".bin")).unwrap();

        let bins = node_modules_bin_dirs(&nested);
        assert_eq!(bins[0], nested.join("node_modules").join(".bin"));
        assert_eq!(bins[1], root.join("node_modules").join(".bin"));
    }

    #[test]
    fn bin_dirs_include_pnpm_hoisted_dir() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        fs::create_dir_all(
            root.join("node_modules")
                .join(".pnpm")
                .join("node_modules")
                .join(".bin"),
        )
        .unwrap();

        let bins = node_modules_bin_dirs(&root);
        assert_eq!(
            bins,
            vec![
                root.join("node_modules")
                    .join(".pnpm")
                    .join("node_modules")
                    .join(".bin")
            ]
        );
    }

    #[test]
    fn resolves_declared_package_bin_from_nearest_package() {
        let dir = tempfile::tempdir().unwrap();
        let pkg = dir.path().join("pkg");
        let nested = pkg.join("src");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(pkg.join("bin")).unwrap();
        fs::write(
            pkg.join("package.json"),
            r#"{"name":"tooling","bin":{"hello":"bin/hello.js"}}"#,
        )
        .unwrap();
        fs::write(pkg.join("bin").join("hello.js"), "console.log('hi')").unwrap();

        let resolved = resolve_declared_package_bin(&nested, "hello").unwrap();
        assert_eq!(resolved, Some(pkg.join("bin").join("hello.js")));
    }

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
