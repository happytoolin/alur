use std::path::{Path, PathBuf};

use crate::core::shell::shell_escape;
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
    print!("{}", render_init(shell, &exe_path, &bin_dir));
    Ok(())
}

pub fn render_init(shell: InitShell, exe_path: &Path, bin_dir: &Path) -> String {
    match shell {
        InitShell::Bash | InitShell::Zsh => render_posix(shell, exe_path, bin_dir),
        InitShell::Fish => render_fish(exe_path, bin_dir),
        InitShell::PowerShell => render_powershell(exe_path, bin_dir),
        InitShell::Nushell => render_nushell(exe_path, bin_dir),
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

fn render_posix(shell: InitShell, exe_path: &Path, bin_dir: &Path) -> String {
    let shell_name = shell.canonical_name();
    let hni_cmd = shell_escape(exe_path.to_string_lossy().as_ref());
    let hni_bin = shell_escape(bin_dir.to_string_lossy().as_ref());

    format!(
        "# hni init for {shell_name}\n\
         _hni_cmd={hni_cmd}\n\
         _hni_bin={hni_bin}\n\
         if ! {{ [ -n \"${{HNI_REAL_NODE:-}}\" ] && [ -e \"${{HNI_REAL_NODE}}\" ]; }}; then\n\
           _hni_real_node=\"$(\"$_hni_cmd\" internal real-node-path)\"\n\
           if [ -n \"$_hni_real_node\" ] && [ -e \"$_hni_real_node\" ]; then\n\
             export HNI_REAL_NODE=\"$_hni_real_node\"\n\
           fi\n\
         fi\n\
         if [ \"${{PATH:-}}\" != \"$_hni_bin\" ] && [ \"${{PATH#\"$_hni_bin:\"}}\" = \"${{PATH}}\" ]; then\n\
           export PATH=\"$_hni_bin${{PATH:+:$PATH}}\"\n\
         fi\n\
         node() {{ {hni_cmd} node \"$@\"; }}\n\
         unset _hni_cmd\n\
         unset _hni_bin\n\
         unset _hni_real_node\n"
    )
}

fn render_fish(exe_path: &Path, bin_dir: &Path) -> String {
    let hni_cmd = fish_quote(exe_path.to_string_lossy().as_ref());
    let hni_bin = fish_quote(bin_dir.to_string_lossy().as_ref());

    format!(
        "# hni init for fish\n\
         set -l __hni_cmd {hni_cmd}\n\
         set -l __hni_bin {hni_bin}\n\
         if not set -q HNI_REAL_NODE; or not test -e \"$HNI_REAL_NODE\"\n\
             set -l __hni_real_node (\"$__hni_cmd\" internal real-node-path)\n\
             if test -n \"$__hni_real_node\"; and test -e \"$__hni_real_node\"\n\
                 set -gx HNI_REAL_NODE \"$__hni_real_node\"\n\
             end\n\
         end\n\
         if test (count $PATH) -eq 0\n\
             set -gx PATH \"$__hni_bin\"\n\
         else if test \"$PATH[1]\" != \"$__hni_bin\"\n\
             set -gx PATH \"$__hni_bin\" $PATH\n\
         end\n\
         functions -e node 2>/dev/null\n\
         function node --description \"hni node shim\"\n\
             {hni_cmd} node $argv\n\
         end\n"
    )
}

fn render_powershell(exe_path: &Path, bin_dir: &Path) -> String {
    let hni_cmd = powershell_quote(exe_path.to_string_lossy().as_ref());
    let hni_bin = powershell_quote(bin_dir.to_string_lossy().as_ref());

    format!(
        "# hni init for powershell\n\
         $__hniCmd = {hni_cmd}\n\
         $__hniBin = {hni_bin}\n\
         if (-not ($env:HNI_REAL_NODE -and (Test-Path -LiteralPath $env:HNI_REAL_NODE))) {{\n\
           $__hniRealNode = (& $__hniCmd internal real-node-path).Trim()\n\
           if ($__hniRealNode -and (Test-Path -LiteralPath $__hniRealNode)) {{\n\
             $env:HNI_REAL_NODE = $__hniRealNode\n\
           }}\n\
         }}\n\
         $__hniPathEntries = if ($env:PATH) {{ $env:PATH -split ';' }} else {{ @() }}\n\
         $__hniHasPriority = $__hniPathEntries.Count -gt 0 -and [System.StringComparer]::OrdinalIgnoreCase.Equals($__hniPathEntries[0], $__hniBin)\n\
         if (-not $__hniHasPriority) {{\n\
           $env:PATH = if ($env:PATH) {{ \"$($__hniBin);$env:PATH\" }} else {{ $__hniBin }}\n\
         }}\n\
         function global:node {{\n\
           & {hni_cmd} node @args\n\
         }}\n\
         Remove-Variable __hniCmd, __hniBin, __hniRealNode, __hniPathEntries, __hniHasPriority -ErrorAction SilentlyContinue\n"
    )
}

fn render_nushell(exe_path: &Path, bin_dir: &Path) -> String {
    let hni_cmd = nushell_quote(exe_path.to_string_lossy().as_ref());
    let hni_bin = nushell_quote(bin_dir.to_string_lossy().as_ref());

    format!(
        "# hni init for nushell\n\
         let hni_cmd = {hni_cmd}\n\
         let hni_bin = {hni_bin}\n\
         if (not ($env.HNI_REAL_NODE? | default '' | path exists)) {{\n\
           let hni_real_node = (^$hni_cmd internal real-node-path | str trim)\n\
           if (not ($hni_real_node | is-empty)) and ($hni_real_node | path exists) {{\n\
             $env.HNI_REAL_NODE = $hni_real_node\n\
           }}\n\
         }}\n\
         if (($env.PATH | is-empty) or (($env.PATH | first) != $hni_bin)) {{\n\
           $env.PATH = ($env.PATH | prepend $hni_bin)\n\
         }}\n\
         def --wrapped node [...rest] {{\n\
           ^{hni_cmd} node ...$rest\n\
         }}\n"
    )
}

fn fish_quote(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
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
    fn posix_render_embeds_absolute_paths_and_dedupe_check() {
        let out = render_init(
            InitShell::Bash,
            Path::new("/tmp/hni/bin/hni"),
            Path::new("/tmp/hni/bin"),
        );
        assert!(out.contains("/tmp/hni/bin/hni"));
        assert!(out.contains("/tmp/hni/bin"));
        assert!(out.contains("internal real-node-path"));
        assert!(out.contains("PATH#\"$_hni_bin:\""));
        assert!(out.contains("node() {"));
    }

    #[test]
    fn fish_render_uses_path_first_element_check() {
        let out = render_init(
            InitShell::Fish,
            Path::new("/tmp/hni/bin/hni"),
            Path::new("/tmp/hni/bin"),
        );
        assert!(out.contains("set -gx PATH"));
        assert!(out.contains("$PATH[1]"));
        assert!(out.contains("internal real-node-path"));
        assert!(out.contains("function node"));
    }

    #[test]
    fn powershell_render_sets_env_and_path() {
        let out = render_init(
            InitShell::PowerShell,
            Path::new("C:/hni/bin/hni.exe"),
            Path::new("C:/hni/bin"),
        );
        assert!(out.contains("$env:HNI_REAL_NODE"));
        assert!(out.contains("[System.StringComparer]::OrdinalIgnoreCase"));
        assert!(out.contains("internal real-node-path"));
        assert!(out.contains("function global:node"));
    }

    #[test]
    fn nushell_render_uses_double_quoted_strings_and_prepend() {
        let out = render_init(
            InitShell::Nushell,
            Path::new("/tmp/hni/bin/hni"),
            Path::new("/tmp/hni/bin"),
        );
        assert!(out.contains("let hni_cmd = \""));
        assert!(out.contains("| prepend $hni_bin"));
        assert!(out.contains("^$hni_cmd internal real-node-path"));
        assert!(out.contains("def --wrapped node"));
        assert!(out.contains("^\"/tmp/hni/bin/hni\" node ...$rest"));
    }

    #[test]
    fn nushell_quote_uses_double_quoted_strings() {
        assert_eq!(
            nushell_quote(r#"C:\hni\bin\hni "dev".exe"#),
            r#""C:\\hni\\bin\\hni \"dev\".exe""#
        );
    }
}
