use std::io;

use anyhow::{Result, anyhow};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};

use super::cli::hni_command;

/// Print shell completion script.
///
/// # Errors
///
/// Returns an error if:
/// - Shell is not provided and cannot be detected from environment
/// - Shell is not one of: bash, zsh, fish
pub fn print_completion(shell: Option<&str>, program: &str) -> Result<()> {
    let shell = shell
        .map(str::to_owned)
        .or_else(detect_shell_from_env)
        .ok_or_else(|| anyhow!("parse error: missing shell; use one of: bash, zsh, fish"))?;

    let mut cmd = hni_command();
    let mut out = io::stdout();

    match shell.as_str() {
        "bash" => generate(Bash, &mut cmd, program, &mut out),
        "zsh" => generate(Zsh, &mut cmd, program, &mut out),
        "fish" => generate(Fish, &mut cmd, program, &mut out),
        _ => {
            return Err(anyhow!(
                "parse error: unsupported shell '{shell}'; use: bash, zsh, fish"
            ));
        }
    }

    Ok(())
}

fn detect_shell_from_env() -> Option<String> {
    let shell = std::env::var("SHELL").ok()?;
    let name = std::path::Path::new(&shell)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)?;
    Some(name.to_ascii_lowercase())
}
