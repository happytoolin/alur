use clap::{Arg, ArgAction, Command, builder::PossibleValuesParser, value_parser};

use crate::app::{
    cli::alur_command,
    command_registry::{CommandSpec, help_command_for_topic},
    init::SUPPORTED_SHELL_NAMES,
};
use crate::core::types::HelpTopic;

pub fn print_help(topic: HelpTopic) {
    let mut cmd = help_command_for_topic(topic);
    let _ = cmd.print_long_help();
    println!();
}

pub fn top_level_help() -> Command {
    alur_command()
}

pub fn command_help(spec: &CommandSpec) -> Command {
    with_global_flags(
        Command::new(spec.name)
            .about(spec.about)
            .long_about(spec.long_about)
            .arg(command_args_arg())
            .after_help(spec.examples),
    )
}

pub fn init_help() -> Command {
    with_global_flags(
        Command::new("init")
            .about("print shell init code for node shim")
            .long_about(
                "Creates alur's managed node shim symlink and prints shell-specific PATH setup.\n\
                 Add the generated line at the end of your shell config, after nvm/mise/asdf/fnm/volta init.",
            )
            .arg(init_shell_arg())
            .after_help(
                "Examples:\n\
                 \n\
                 alur init bash\n\
                 alur init zsh\n\
                 alur init fish\n\
                 alur init powershell\n\
                 alur init nushell",
            ),
    )
}

fn with_global_flags(cmd: Command) -> Command {
    cmd.disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("print-command")
                .long("print-command")
                .help("print resolved command and exit")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("explain")
                .long("explain")
                .help("print detection + resolution details and exit")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("fast")
                .long("fast")
                .help("prefer fast mode for eligible run/exec commands")
                .conflicts_with("pm")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("pm")
                .long("pm")
                .help("force package-manager mode for this invocation")
                .conflicts_with("fast")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cwd")
                .short('C')
                .value_name("DIR")
                .help("run as if in <dir>")
                .value_parser(value_parser!(std::path::PathBuf))
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("version")
                .short('v')
                .long("version")
                .help("show versions")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .help("show help")
                .action(ArgAction::SetTrue),
        )
}

pub fn command_args_arg() -> Arg {
    Arg::new("args")
        .value_name("ARGS")
        .help("arguments forwarded to the resolved command")
        .num_args(0..)
        .allow_hyphen_values(true)
        .action(ArgAction::Append)
}

fn init_shell_arg() -> Arg {
    Arg::new("shell")
        .value_name("SHELL")
        .help("shell to initialize")
        .required(true)
        .value_parser(PossibleValuesParser::new(SUPPORTED_SHELL_NAMES))
}
