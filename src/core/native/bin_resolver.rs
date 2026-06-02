use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::Result;

use crate::core::{types::NativeLocalBinLauncher, util::has_unix_executable_bit};

use super::shim_parser::{NodeBinLaunch, node_args_from_shebang, parse_node_shell_shim};

pub(super) fn resolve_local_bin_launcher(bin_path: &Path) -> Result<NativeLocalBinLauncher> {
    crate::core::profile::measure("local_bin.launcher", || {
        let inspected_path = resolve_bin_source_path(bin_path)?;
        let extension = inspected_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());

        match extension.as_deref() {
            Some("cmd") | Some("bat") => {
                return Ok(NativeLocalBinLauncher::Cmd(inspected_path));
            }
            Some("ps1") => return Ok(NativeLocalBinLauncher::PowerShell(inspected_path)),
            Some("js") | Some("cjs") | Some("mjs") => {
                return Ok(NativeLocalBinLauncher::NodeScript {
                    script_path: inspected_path,
                    node_args: Vec::new(),
                });
            }
            _ => {}
        }

        if has_unix_executable_bit(&inspected_path) {
            return Ok(NativeLocalBinLauncher::Binary(inspected_path));
        }

        if let Some(node_launch) = detect_node_launcher_without_extension(&inspected_path)? {
            return Ok(NativeLocalBinLauncher::NodeScript {
                script_path: node_launch.script_path,
                node_args: node_launch.node_args,
            });
        }

        Ok(NativeLocalBinLauncher::Binary(inspected_path))
    })
}

fn detect_node_launcher_without_extension(inspected_path: &Path) -> Result<Option<NodeBinLaunch>> {
    let raw = match crate::core::profile::measure("local_bin.read_launcher", || {
        read_launcher_prefix(inspected_path)
    }) {
        Ok(raw) => raw,
        Err(_) => return Ok(None),
    };

    if let Some(node_args) = raw.lines().next().and_then(node_args_from_shebang) {
        return Ok(Some(NodeBinLaunch {
            script_path: inspected_path.to_path_buf(),
            node_args,
        }));
    }

    crate::core::profile::measure("local_bin.parse_shell_shim", || {
        Ok(parse_node_shell_shim(&raw, inspected_path))
    })
}

fn read_launcher_prefix(path: &Path) -> std::io::Result<String> {
    const MAX_LAUNCHER_BYTES: u64 = 8 * 1024;

    let mut raw = String::new();
    fs::File::open(path)?
        .take(MAX_LAUNCHER_BYTES)
        .read_to_string(&mut raw)?;
    Ok(raw)
}

fn resolve_bin_source_path(bin_path: &Path) -> Result<PathBuf> {
    crate::core::profile::measure("local_bin.resolve_source", || {
        let mut current = bin_path.to_path_buf();
        let mut followed_symlink = false;

        for _ in 0..8 {
            let metadata = match fs::symlink_metadata(&current) {
                Ok(metadata) => metadata,
                Err(_) => return Ok(current),
            };

            if !metadata.file_type().is_symlink() {
                return if followed_symlink {
                    Ok(dunce::canonicalize(&current).unwrap_or(current))
                } else {
                    Ok(current)
                };
            }

            let target = fs::read_link(&current)?;
            followed_symlink = true;
            current = if target.is_absolute() {
                target
            } else {
                current
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(target)
            };
        }

        Ok(dunce::canonicalize(&current).unwrap_or(current))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::NativeLocalBinLauncher;
    use std::fs;

    #[cfg(unix)]
    #[test]
    fn resolves_symlinked_js_bins_to_node_script_launcher() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let package_dir = dir.path().join("node_modules").join("tool");
        let bin_dir = dir.path().join("node_modules").join(".bin");
        fs::create_dir_all(&package_dir).unwrap();
        fs::create_dir_all(&bin_dir).unwrap();
        let script = package_dir.join("cli.js");
        fs::write(&script, "console.log('hi')").unwrap();
        let shim = bin_dir.join("tool");
        symlink("../tool/cli.js", &shim).unwrap();

        let launcher = resolve_local_bin_launcher(&shim).unwrap();
        assert_eq!(
            launcher,
            NativeLocalBinLauncher::NodeScript {
                script_path: dunce::canonicalize(&script).unwrap(),
                node_args: Vec::new(),
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn executable_extensionless_bins_run_directly() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("hello");
        fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
        let mut permissions = fs::metadata(&bin).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&bin, permissions).unwrap();

        let launcher = resolve_local_bin_launcher(&bin).unwrap();
        assert_eq!(launcher, NativeLocalBinLauncher::Binary(bin));
    }
}
