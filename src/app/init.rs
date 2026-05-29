use std::path::{Path, PathBuf};

use crate::{core::shell::shell_escape, platform::node::write_real_node_cache};
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
    let (exe_path, bin_dir) = current_binary_paths()?;

    let shim_dir = ensure_node_shim(&exe_path, &bin_dir);

    if let Ok(real_node_path) = crate::platform::node::resolve_real_node_path() {
        let _ = write_real_node_cache(&real_node_path);
    }

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

fn current_binary_paths() -> Result<(PathBuf, PathBuf)> {
    let exe_path = std::env::current_exe().map_err(|error| {
        anyhow!("execution error: failed to determine current executable path: {error}")
    })?;
    let exe_path = dunce::canonicalize(&exe_path).unwrap_or(exe_path);
    let bin_dir = exe_path.parent().map(Path::to_path_buf).ok_or_else(|| {
        anyhow!("execution error: failed to determine current executable directory")
    })?;
    Ok((exe_path, bin_dir))
}

fn ensure_node_shim(exe_path: &Path, bin_dir: &Path) -> PathBuf {
    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    let node_path = bin_dir.join(node_name);

    if try_create_symlink(exe_path, &node_path) {
        return bin_dir.to_path_buf();
    }

    let Some(config_dir) = dirs::config_dir() else {
        eprintln!("[hni] warning: cannot create node shim — unable to determine config directory");
        return bin_dir.to_path_buf();
    };

    let managed_dir = config_dir.join("hni").join("bin");
    let managed_node = managed_dir.join(node_name);

    if managed_node.exists() {
        return managed_dir;
    }

    if std::fs::create_dir_all(&managed_dir).is_err() {
        eprintln!(
            "[hni] warning: cannot create managed shim directory at {}",
            managed_dir.display()
        );
        return bin_dir.to_path_buf();
    }

    if !try_create_symlink(exe_path, &managed_node) {
        eprintln!(
            "[hni] warning: cannot create node shim symlink at {}",
            managed_node.display()
        );
        return bin_dir.to_path_buf();
    }

    managed_dir
}

fn try_create_symlink(target: &Path, link: &Path) -> bool {
    if link.exists() {
        return true;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }

    #[cfg(windows)]
    {
        if target.extension().map_or(false, |ext| ext == "exe") {
            std::os::windows::fs::symlink_file(target, link).is_ok()
        } else {
            std::os::windows::fs::symlink_file(&target.with_extension("exe"), link).is_ok()
        }
    }
}

fn render_posix(path_dir: &Path) -> String {
    let hni_path = shell_escape(path_dir.to_string_lossy().as_ref());

    format!(
        "# hni init\n\
         case \":${{PATH:-}}\" in\n\
           *\":{hni_path}:\"*) ;;\n\
           *) export PATH=\"{hni_path}:${{PATH:-}}\" ;;\n\
         esac\n"
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
