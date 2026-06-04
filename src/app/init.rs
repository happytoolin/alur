use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use crate::platform::paths_equal;
use crate::{
    core::shell::shell_escape,
    platform::node::{managed_node_shim_dir, node_binary_name},
};
use anyhow::{Context, Result, anyhow};

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
    let exe_path = std::env::current_exe()
        .context("execution error: failed to determine current executable path")?;
    Ok(dunce::canonicalize(&exe_path).unwrap_or(exe_path))
}

fn ensure_node_shim(exe_path: &Path) -> Result<PathBuf> {
    let managed_dir = managed_node_shim_dir().ok_or_else(|| {
        anyhow!("execution error: unable to determine managed alur shim directory")
    })?;
    let managed_node = managed_dir.join(node_binary_name());

    fs::create_dir_all(&managed_dir).with_context(|| {
        format!(
            "execution error: failed to create managed alur shim directory at {}",
            managed_dir.display()
        )
    })?;

    if node_shim_matches_current(&managed_node, exe_path)? {
        return Ok(managed_dir);
    }

    if fs::symlink_metadata(&managed_node).is_ok() {
        remove_node_shim(&managed_node)?;
    }

    create_node_shim(exe_path, &managed_node).with_context(|| {
        format!(
            "execution error: failed to create managed node shim at {}",
            managed_node.display()
        )
    })?;

    if !node_shim_matches_current(&managed_node, exe_path)? {
        return Err(anyhow!(
            "execution error: managed node shim was created but does not target current alur: {}",
            managed_node.display()
        ));
    }

    Ok(managed_dir)
}

fn node_shim_matches_current(path: &Path, expected_target: &Path) -> Result<bool> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(false);
    };

    #[cfg(unix)]
    {
        Ok(metadata.file_type().is_symlink() && node_symlink_points_to(path, expected_target))
    }

    #[cfg(windows)]
    {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Ok(false);
        }

        files_have_same_contents(path, expected_target)
    }
}

#[cfg(unix)]
fn node_symlink_points_to(link: &Path, expected_target: &Path) -> bool {
    fs::read_link(link)
        .ok()
        .map(|target| {
            if target.is_absolute() {
                target
            } else {
                link.parent()
                    .map(|parent| parent.join(&target))
                    .unwrap_or(target)
            }
        })
        .is_some_and(|target| paths_equal(&target, expected_target))
}

fn remove_node_shim(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect managed node shim: {}", path.display()))?;

    if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove managed node shim dir: {}", path.display()))
    } else {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove managed node shim: {}", path.display()))
    }
}

fn create_node_shim(target: &Path, link: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    {
        fs::copy(target, link).map(|_| ())
    }
}

#[cfg(windows)]
fn files_have_same_contents(left: &Path, right: &Path) -> Result<bool> {
    let left_metadata = fs::metadata(left)
        .with_context(|| format!("failed to inspect node launcher: {}", left.display()))?;
    let right_metadata = fs::metadata(right)
        .with_context(|| format!("failed to inspect alur launcher: {}", right.display()))?;

    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }

    let left_contents = fs::read(left)
        .with_context(|| format!("failed to read node launcher: {}", left.display()))?;
    let right_contents = fs::read(right)
        .with_context(|| format!("failed to read alur launcher: {}", right.display()))?;

    Ok(left_contents == right_contents)
}

fn render_posix(path_dir: &Path) -> String {
    let alur_path = shell_escape(path_dir.to_string_lossy().as_ref());

    format!(
        "# alur init\n\
         _alur_path={alur_path}\n\
         if [ \"${{PATH:-}}\" != \"$_alur_path\" ] && [ \"${{PATH#\"$_alur_path:\"}}\" = \"${{PATH}}\" ]; then\n\
           export PATH=\"$_alur_path${{PATH:+:$PATH}}\"\n\
         fi\n\
         unset _alur_path\n"
    )
}

fn render_fish(path_dir: &Path) -> String {
    let alur_path = fish_quote(path_dir.to_string_lossy().as_ref());

    format!(
        "# alur init for fish\n\
         if test (count $PATH) -eq 0\n\
             set -gx PATH {alur_path}\n\
         else if test \"$PATH[1]\" != {alur_path}\n\
             set -gx PATH {alur_path} $PATH\n\
         end\n"
    )
}

fn render_powershell(path_dir: &Path) -> String {
    let alur_path = powershell_quote(path_dir.to_string_lossy().as_ref());

    format!(
        "# alur init for powershell\n\
         $__alurPath = {alur_path}\n\
         $__alurPathEntries = if ($env:PATH) {{ $env:PATH -split ';' }} else {{ @() }}\n\
         $__alurHasPriority = $__alurPathEntries.Count -gt 0 -and [System.StringComparer]::OrdinalIgnoreCase.Equals($__alurPathEntries[0], $__alurPath)\n\
         if (-not $__alurHasPriority) {{\n\
           $env:PATH = if ($env:PATH) {{ \"$($__alurPath);$env:PATH\" }} else {{ $__alurPath }}\n\
         }}\n\
         Remove-Variable __alurPath, __alurPathEntries, __alurHasPriority -ErrorAction SilentlyContinue\n"
    )
}

fn render_nushell(path_dir: &Path) -> String {
    let alur_path = nushell_quote(path_dir.to_string_lossy().as_ref());

    format!(
        "# alur init for nushell\n\
         let alur_path = {alur_path}\n\
         if (($env.PATH | is-empty) or (($env.PATH | first) != $alur_path)) {{\n\
           $env.PATH = ($env.PATH | prepend $alur_path)\n\
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
        let out = render_init(InitShell::Bash, Path::new("/tmp/alur/bin"));
        assert!(out.contains("export PATH="));
        assert!(out.contains("/tmp/alur/bin"));
        assert!(out.contains("PATH#\"$_alur_path:\""));
        assert!(!out.contains("node()"));
        assert!(!out.contains("ALUR_REAL_NODE"));
        assert!(!out.contains("real-node-path"));
    }

    #[test]
    fn fish_render_is_path_only() {
        let out = render_init(InitShell::Fish, Path::new("/tmp/alur/bin"));
        assert!(out.contains("set -gx PATH"));
        assert!(out.contains("/tmp/alur/bin"));
        assert!(out.contains("$PATH[1]"));
        assert!(!out.contains("function node"));
        assert!(!out.contains("ALUR_REAL_NODE"));
    }

    #[test]
    fn powershell_render_is_path_only() {
        let out = render_init(InitShell::PowerShell, Path::new("C:/alur/bin"));
        assert!(out.contains("$env:PATH"));
        assert!(out.contains("C:/alur/bin"));
        assert!(out.contains("[System.StringComparer]::OrdinalIgnoreCase"));
        assert!(!out.contains("function global:node"));
        assert!(!out.contains("ALUR_REAL_NODE"));
    }

    #[test]
    fn nushell_render_is_path_only() {
        let out = render_init(InitShell::Nushell, Path::new("/tmp/alur/bin"));
        assert!(out.contains("prepend $alur_path"));
        assert!(out.contains("/tmp/alur/bin"));
        assert!(!out.contains("def --wrapped node"));
        assert!(!out.contains("ALUR_REAL_NODE"));
    }

    #[test]
    fn nushell_quote_uses_double_quoted_strings() {
        assert_eq!(
            nushell_quote(r#"C:\alur\bin\alur "dev".exe"#),
            r#""C:\\alur\\bin\\alur \"dev\".exe""#
        );
    }
}
