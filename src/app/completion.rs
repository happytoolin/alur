use std::{env, io};

use anyhow::{Result, anyhow};
use clap::{Arg, ArgAction};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};

use super::command_registry::help_command_for_topic;
use crate::{
    core::{resolve::ResolveContext, types::HelpTopic},
    features::interactive::{completion::completion_candidates, nr_scripts::read_scripts},
};

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

#[must_use]
pub(super) fn nr_completion_script_for(flag: &str) -> Option<String> {
    match flag {
        "--completion-bash" => Some(generate_nr_completion("nr", Bash)),
        "--completion-zsh" => Some(generate_nr_completion("nr", Zsh)),
        "--completion-fish" => Some(generate_nr_completion("nr", Fish)),
        _ => None,
    }
}

pub(super) fn print_nr_completion_query(args: &[String], ctx: &ResolveContext) -> Result<()> {
    let scripts = read_scripts(ctx)?;
    let script_names = scripts.into_iter().map(|script| script.name);

    let comp_word = env::var("COMP_CWORD")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);

    let prefix = if comp_word > 1 {
        args.last().cloned().unwrap_or_default()
    } else {
        args.get(1).cloned().unwrap_or_default()
    };

    for candidate in completion_candidates(&prefix, script_names) {
        println!("{candidate}");
    }

    Ok(())
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
