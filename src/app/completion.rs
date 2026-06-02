use std::io;

use anyhow::{Result, anyhow};
use clap::{Arg, ArgAction};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};

use super::command_registry::help_command_for_topic;
pub use crate::core::types::HelpTopic;

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

    let mut cmd = help_command_for_topic(HelpTopic::Hni);
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

pub fn nr_completion_script_for(flag: &str) -> Option<String> {
    match flag {
        "--completion-bash" => Some(generate_nr_completion("nr", Bash)),
        "--completion-zsh" => Some(generate_nr_completion("nr", Zsh)),
        "--completion-fish" => Some(generate_nr_completion("nr", Fish)),
        _ => None,
    }
}

fn generate_nr_completion<G>(command: &str, generator: G) -> String
where
    G: clap_complete::Generator,
{
    let mut cmd = help_command_for_topic(HelpTopic::Nr)
        .arg(
            Arg::new("completion")
                .long("completion")
                .hide(true)
                .num_args(0..)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("completion-bash")
                .long("completion-bash")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("completion-zsh")
                .long("completion-zsh")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("completion-fish")
                .long("completion-fish")
                .action(ArgAction::SetTrue),
        );

    let mut output = Vec::new();
    generate(generator, &mut cmd, command, &mut output);
    String::from_utf8_lossy(&output).into_owned()
}

fn detect_shell_from_env() -> Option<String> {
    let shell = std::env::var("SHELL").ok()?;
    let name = std::path::Path::new(&shell)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)?;
    Some(name.to_ascii_lowercase())
}
