use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    core::shell::shell_escape,
    platform::{
        node::{managed_node_shim_dir, node_binary_name},
        paths_equal,
    },
};
use anyhow::{Result, anyhow};

pub const SUPPORTED_SHELL_NAMES: &[&str] =
    &["bash", "zsh", "fish", "powershell", "pwsh", "nushell", "nu"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Nushell,
}

impl InitShell {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bash" => Ok(Self::Bash),
            "zsh" => Ok(Self::Zsh),
            "fish" => Ok(Self::Fish),
            "powershell" | "pwsh" => Ok(Self::PowerShell),
            "nushell" | "nu" => Ok(Self::Nushell),
            _ => Err(anyhow!(
                "parse error: unsupported init shell '{value}'; use: {}",
                SUPPORTED_SHELL_NAMES.join(", ")
            )),
        }
    }

    pub fn canonical_name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::PowerShell => "powershell",
            Self::Nushell => "nushell",
        }
    }
}

pub fn print_init(shell_name: &str) -> Result<()> {
    let shell = InitShell::parse(shell_name)?;
    let exe_path = current_binary_path()?;
    let shim_dir = ensure_node_shim(&exe_path)?;

    print!("{}", render_init(shell, &shim_dir));
    Ok(())
}

pub fn render_init(shell: InitShell, path_dir: &Path) -> String {
    match shell {
        InitShell::Bash | InitShell::Zsh => render_posix(path_dir),
        InitShell::Fish => render_fish(path_dir),
        InitShell::PowerShell => render_powershell(path_dir),
        InitShell::Nushell => render_nushell(path_dir),
    }
}

fn current_binary_path() -> Result<PathBuf> {
    let exe_path = std::env::current_exe().map_err(|error| {
        anyhow!("execution error: failed to determine current executable path: {error}")
    })?;
    Ok(dunce::canonicalize(&exe_path).unwrap_or(exe_path))
}

fn ensure_node_shim(exe_path: &Path) -> Result<PathBuf> {
    let managed_dir = managed_node_shim_dir().ok_or_else(|| {
        anyhow!("execution error: unable to determine managed hni shim directory")
    })?;
    let managed_node = managed_dir.join(node_binary_name());

    fs::create_dir_all(&managed_dir).map_err(|error| {
        anyhow!(
            "execution error: failed to create managed hni shim directory at {}: {error}",
            managed_dir.display()
        )
    })?;

    if path_exists_or_symlink(&managed_node) {
        if node_shim_points_to(&managed_node, exe_path) {
            return Ok(managed_dir);
        }

        return Err(anyhow!(
            "execution error: managed node shim already exists and is not an hni symlink: {}",
            managed_node.display()
        ));
    }

    create_node_symlink(exe_path, &managed_node).map_err(|error| {
        anyhow!(
            "execution error: failed to create node shim symlink at {}: {error}",
            managed_node.display()
        )
    })?;

    Ok(managed_dir)
}

fn path_exists_or_symlink(path: &Path) -> bool {
    path.exists() || fs::symlink_metadata(path).is_ok()
}

fn node_shim_points_to(link: &Path, expected_target: &Path) -> bool {
    let Ok(metadata) = fs::symlink_metadata(link) else {
        return false;
    };
    if !metadata.file_type().is_symlink() {
        return false;
    }

    let Ok(target) = fs::read_link(link) else {
        return false;
    };
    let target = if target.is_absolute() {
        target
    } else {
        link.parent()
            .map(|parent| parent.join(&target))
            .unwrap_or(target)
    };

    paths_equal(&target, expected_target)
}

fn create_node_symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link)
    }
}

fn render_posix(path_dir: &Path) -> String {
    let hni_path = shell_escape(path_dir.to_string_lossy().as_ref());

    format!(
        "# hni init\n\
         _hni_path={hni_path}\n\
         case \":${{PATH:-}}:\" in\n\
           *\":$_hni_path:\"*) ;;\n\
           *) export PATH=\"$_hni_path${{PATH:+:$PATH}}\" ;;\n\
         esac\n\
         unset _hni_path\n"
    )
}

fn render_fish(path_dir: &Path) -> String {
    let hni_path = fish_quote(path_dir.to_string_lossy().as_ref());

    format!(
        "# hni init for fish\n\
         if test (count $PATH) -eq 0\n\
             set -gx PATH {hni_path}\n\
         else if not contains {hni_path} $PATH\n\
             set -gx PATH {hni_path} $PATH\n\
         end\n"
    )
}

fn render_powershell(path_dir: &Path) -> String {
    let hni_path = powershell_quote(path_dir.to_string_lossy().as_ref());

    format!(
        "# hni init for powershell\n\
         $__hniPath = {hni_path}\n\
         $__hniPathEntries = if ($env:PATH) {{ $env:PATH -split ';' }} else {{ @() }}\n\
         $__hniHasEntry = $__hniPathEntries -contains $__hniPath\n\
         if (-not $__hniHasEntry) {{\n\
           $env:PATH = if ($env:PATH) {{ \"$($__hniPath);$env:PATH\" }} else {{ $__hniPath }}\n\
         }}\n\
         Remove-Variable __hniPath, __hniPathEntries, __hniHasEntry -ErrorAction SilentlyContinue\n"
    )
}

fn render_nushell(path_dir: &Path) -> String {
    let hni_path = nushell_quote(path_dir.to_string_lossy().as_ref());

    format!(
        "# hni init for nushell\n\
         let hni_path = {hni_path}\n\
         if (($env.PATH | is-empty) or (not ($env.PATH | any {{|p| $p == $hni_path}}))) {{\n\
           $env.PATH = ($env.PATH | prepend $hni_path)\n\
         }}\n"
    )
}

fn fish_quote(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn nushell_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_shell_aliases() {
        assert_eq!(InitShell::parse("bash").unwrap(), InitShell::Bash);
        assert_eq!(InitShell::parse("pwsh").unwrap(), InitShell::PowerShell);
        assert_eq!(InitShell::parse("nu").unwrap(), InitShell::Nushell);
    }

    #[test]
    fn rejects_unsupported_shells() {
        let err = InitShell::parse("tcsh").unwrap_err();
        assert!(err.to_string().contains("unsupported init shell"));
    }

    #[test]
    fn posix_render_is_path_only() {
        let out = render_init(InitShell::Bash, Path::new("/tmp/hni/bin"));
        assert!(out.contains("export PATH="));
        assert!(out.contains("/tmp/hni/bin"));
        assert!(out.contains("case"));
        assert!(!out.contains("node()"));
        assert!(!out.contains("HNI_REAL_NODE"));
        assert!(!out.contains("real-node-path"));
    }

    #[test]
    fn fish_render_is_path_only() {
        let out = render_init(InitShell::Fish, Path::new("/tmp/hni/bin"));
        assert!(out.contains("set -gx PATH"));
        assert!(out.contains("/tmp/hni/bin"));
        assert!(!out.contains("function node"));
        assert!(!out.contains("HNI_REAL_NODE"));
    }

    #[test]
    fn powershell_render_is_path_only() {
        let out = render_init(InitShell::PowerShell, Path::new("C:/hni/bin"));
        assert!(out.contains("$env:PATH"));
        assert!(out.contains("C:/hni/bin"));
        assert!(!out.contains("function global:node"));
        assert!(!out.contains("HNI_REAL_NODE"));
    }

    #[test]
    fn nushell_render_is_path_only() {
        let out = render_init(InitShell::Nushell, Path::new("/tmp/hni/bin"));
        assert!(out.contains("prepend $hni_path"));
        assert!(out.contains("/tmp/hni/bin"));
        assert!(!out.contains("def --wrapped node"));
        assert!(!out.contains("HNI_REAL_NODE"));
    }

    #[test]
    fn nushell_quote_uses_double_quoted_strings() {
        assert_eq!(
            nushell_quote(r#"C:\hni\bin\hni "dev".exe"#),
            r#""C:\\hni\\bin\\hni \"dev\".exe""#
        );
    }
}
