use clap::{Arg, ArgAction, Command, builder::PossibleValuesParser, value_parser};

use crate::app::{
    command_registry::{CommandSpec, command_subcommands, help_command_for_topic},
    init::SUPPORTED_SHELL_NAMES,
};
use crate::core::types::HelpTopic;

pub fn print_help(topic: HelpTopic) {
    let mut cmd = help_command_for_topic(topic);
    let _ = cmd.print_long_help();
    println!();
}

pub fn top_level_help() -> Command {
    let mut cmd = with_global_flags(
        Command::new("hni")
            .about("use the right package manager")
            .long_about(
                "hni is a multicall package-manager router.\n\
                 It powers commands like ni, nr, nlx, nru, nun, nci, na, np, ns, and node.\n\
                 Fast mode is the default for eligible nr and nlx commands.",
            )
            .subcommand(Command::new("init").about("print shell init code"))
            .subcommand(Command::new("doctor").about("print environment and detection diagnostics"))
            .subcommand(Command::new("completion").about("print shell completion script"))
            .after_help(
                "Quick examples:\n\
                 \n\
                 ni vite\n\
                 ni --explain react -D\n\
                 nr dev\n\
                 nr --pm dev\n\
                 nr dev -- --port=3000\n\
                 nlx create-vite@latest\n\
                 nru --interactive\n\
                 nun --multi-select\n\
                 np \"echo one\" \"echo two\"\n\
                 ns \"npm run build\" \"npm run test\"\n\
                 hni init bash\n\
                 hni doctor\n\
                 hni help ni\n\
                 hni completion zsh\n\
                 node install react",
            ),
    );

    for subcommand in command_subcommands() {
        cmd = cmd.subcommand(subcommand);
    }

    cmd
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
                "Creates hni's managed node shim symlink and prints shell-specific PATH setup.\n\
                 Add the generated line at the end of your shell config, after nvm/mise/asdf/fnm/volta init.",
            )
            .arg(init_shell_arg())
            .after_help(
                "Examples:\n\
                 \n\
                 hni init bash\n\
                 hni init zsh\n\
                 hni init fish\n\
                 hni init powershell\n\
                 hni init nushell",
            ),
    )
}

fn with_global_flags(cmd: Command) -> Command {
    cmd.disable_help_flag(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("debug")
                .short('?')
                .long("debug-resolved")
                .help("print resolved command and exit (aliases: --dry-run, --print-command)")
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
