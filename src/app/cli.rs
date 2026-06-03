use std::{env, ffi::OsStr, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::{
    ArgAction, Args, Command, CommandFactory, Parser, Subcommand, builder::PossibleValuesParser,
};

use crate::app::{
    command_registry::{help_topic_by_name, help_topic_for_invocation, invocation_from_name},
    help::command_args_arg,
    init::SUPPORTED_SHELL_NAMES,
};
use crate::core::types::{HelpTopic, InvocationKind};

const HNI_AFTER_HELP: &str = "Quick examples:\n\
\n\
hni install vite\n\
hni install --explain react -D\n\
hni uninstall lodash\n\
hni run dev\n\
hni run --pm dev\n\
hni run dev -- --port=3000\n\
hni exec create-vite@latest\n\
np \"echo one\" \"echo two\"\n\
ns \"npm run build\" \"npm run test\"\n\
hni init bash\n\
hni doctor\n\
hni help ni\n\
hni completion zsh\n\
node install react";

#[derive(Debug, Clone)]
pub struct ParsedInvocation {
    pub cwd: PathBuf,
    pub print_command: bool,
    pub explain: bool,
    pub fast_override: Option<bool>,
    pub command: ParsedCommand,
}

#[derive(Debug, Clone)]
pub enum ParsedCommand {
    PrintHelp(HelpTopic),
    PrintVersions,
    Doctor,
    Completion {
        shell: Option<String>,
        program: String,
    },
    Init {
        shell: String,
    },
    InternalRealNodePath,
    InternalProfileLoop {
        invocation: InvocationKind,
        args: Vec<String>,
        iterations: usize,
        timings: bool,
    },
    Execute {
        invocation: InvocationKind,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
struct SharedFlags {
    cwd: Vec<PathBuf>,
    print_command: bool,
    explain: bool,
    fast_override: Option<bool>,
    help: bool,
    version: bool,
}

#[derive(Debug, Parser)]
#[command(
    name = "hni",
    about = "use the right package manager",
    long_about = "hni is a multicall package-manager router.\nIt powers hni install/uninstall/run/exec/ci/parallel/sequential plus ni, nr, nlx, nun, nci, np, ns, and node.\nFast mode is the default for eligible nr and nlx commands.",
    after_help = HNI_AFTER_HELP,
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
struct HniCli {
    #[command(flatten)]
    _shared: ClapSharedFlags,
    #[command(subcommand)]
    command: Option<HniSubcommand>,
}

#[derive(Debug, Parser)]
#[command(
    name = "hni",
    about = "use the right package manager",
    long_about = "hni is a multicall package-manager router.\nIt powers hni install/uninstall/run/exec/ci/parallel/sequential plus ni, nr, nlx, nun, nci, np, ns, and node.\nFast mode is the default for eligible nr and nlx commands.",
    after_help = HNI_AFTER_HELP,
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
struct HniPublicCli {
    #[command(flatten)]
    _shared: ClapSharedFlags,
    #[command(subcommand)]
    _command: Option<HniPublicSubcommand>,
}

#[derive(Debug, Args)]
struct ClapSharedFlags {
    #[arg(long, global = true, help = "print resolved command and exit")]
    print_command: bool,
    #[arg(
        long,
        global = true,
        help = "print detection + resolution details and exit"
    )]
    explain: bool,
    #[arg(
        long,
        global = true,
        conflicts_with = "pm",
        help = "prefer fast mode for eligible run/exec commands"
    )]
    fast: bool,
    #[arg(
        long,
        global = true,
        conflicts_with = "fast",
        help = "force package-manager mode for this invocation"
    )]
    pm: bool,
    #[arg(
        short = 'C',
        long = "cwd",
        global = true,
        value_name = "DIR",
        value_parser = clap::value_parser!(PathBuf),
        action = ArgAction::Append,
        help = "run as if in <dir>"
    )]
    cwd: Vec<PathBuf>,
    #[arg(short = 'v', long, global = true, help = "show versions")]
    version: bool,
    #[arg(short = 'h', long, global = true, help = "show help")]
    help: bool,
}

#[derive(Debug, Subcommand)]
enum HniSubcommand {
    #[command(about = "install or add dependencies")]
    Install(ForwardedArgs),
    #[command(about = "uninstall dependencies")]
    Uninstall(ForwardedArgs),
    #[command(about = "run package scripts")]
    Run(ForwardedArgs),
    #[command(about = "execute package binaries")]
    Exec(ForwardedArgs),
    #[command(about = "clean install")]
    Ci(ForwardedArgs),
    #[command(about = "run shell commands in parallel")]
    Parallel(ForwardedArgs),
    #[command(about = "run shell commands sequentially")]
    Sequential(ForwardedArgs),
    #[command(about = "print hni or command help")]
    Help(HelpArgs),
    #[command(about = "print environment and detection diagnostics")]
    Doctor,
    #[command(about = "print shell completion script")]
    Completion(CompletionArgs),
    #[command(about = "print shell init code")]
    Init(InitArgs),
    #[command(hide = true)]
    Internal(InternalArgs),
}

#[derive(Debug, Subcommand)]
enum HniPublicSubcommand {
    #[command(about = "install or add dependencies")]
    Install(ForwardedArgs),
    #[command(about = "uninstall dependencies")]
    Uninstall(ForwardedArgs),
    #[command(about = "run package scripts")]
    Run(ForwardedArgs),
    #[command(about = "execute package binaries")]
    Exec(ForwardedArgs),
    #[command(about = "clean install")]
    Ci(ForwardedArgs),
    #[command(about = "run shell commands in parallel")]
    Parallel(ForwardedArgs),
    #[command(about = "run shell commands sequentially")]
    Sequential(ForwardedArgs),
    #[command(about = "print hni or command help")]
    Help(HelpArgs),
    #[command(about = "print environment and detection diagnostics")]
    Doctor,
    #[command(about = "print shell completion script")]
    Completion(CompletionArgs),
    #[command(about = "print shell init code")]
    Init(InitArgs),
}

#[derive(Debug, Args)]
struct ForwardedArgs {
    #[arg(
        value_name = "ARGS",
        num_args = 0..,
        allow_hyphen_values = true,
        help = "arguments forwarded to the resolved command"
    )]
    args: Vec<String>,
}

#[derive(Debug, Args)]
struct CompletionArgs {
    shell: Option<String>,
}

#[derive(Debug, Args)]
struct HelpArgs {
    command: Option<String>,
}

#[derive(Debug, Args)]
struct InitArgs {
    #[arg(value_parser = PossibleValuesParser::new(SUPPORTED_SHELL_NAMES))]
    shell: String,
}

#[derive(Debug, Args)]
struct InternalArgs {
    #[command(subcommand)]
    command: Option<InternalSubcommand>,
}

#[derive(Debug, Subcommand)]
enum InternalSubcommand {
    #[command(hide = true)]
    RealNodePath,
    #[command(hide = true)]
    ProfileLoop(ProfileLoopArgs),
}

#[derive(Debug, Args)]
struct ProfileLoopArgs {
    #[arg(long, default_value_t = 2000)]
    iterations: usize,
    #[arg(long)]
    timings: bool,
    invocation: String,
    #[arg(value_name = "ARGS", num_args = 0.., allow_hyphen_values = true)]
    args: Vec<String>,
}

impl SharedFlags {
    fn into_invocation(self, command: ParsedCommand) -> Result<ParsedInvocation> {
        Ok(ParsedInvocation {
            cwd: resolve_cwd(&self.cwd)?,
            print_command: self.print_command,
            explain: self.explain,
            fast_override: self.fast_override,
            command,
        })
    }
}

/// Parse CLI arguments from the current process environment.
///
/// # Errors
///
/// Returns an error when argv is unavailable, shared flags conflict or use an
/// invalid working directory, clap rejects command arguments, help/internal
/// topics are unknown, or changing to the requested current directory fails.
pub fn parse_from_env() -> Result<ParsedInvocation> {
    let argv = env::args().collect::<Vec<_>>();
    let Some(argv0) = argv.first() else {
        return Err(anyhow!("parse error: missing argv[0]"));
    };

    let invocation = invocation_from_argv0(argv0);
    let (shared_flags, command_args) = extract_shared_flags(&argv[1..])?;

    if invocation == InvocationKind::Hni {
        parse_hni(argv0, &command_args, shared_flags)
    } else {
        parse_alias(invocation, &command_args, shared_flags)
    }
}

fn parse_hni(argv0: &str, args: &[String], shared_flags: SharedFlags) -> Result<ParsedInvocation> {
    let program = normalized_program_name(argv0);
    let mut clap_args = Vec::with_capacity(args.len() + 1);
    clap_args.push(program.clone());
    clap_args.extend(args.iter().cloned());

    let parsed = HniCli::try_parse_from(clap_args).context("parse error")?;

    let mut command = match parsed.command {
        Some(subcommand) => parsed_hni_subcommand(subcommand, program.clone())?,
        None => ParsedCommand::PrintHelp(HelpTopic::Hni),
    };

    if shared_flags.version {
        command = ParsedCommand::PrintVersions;
    } else if shared_flags.help {
        command = ParsedCommand::PrintHelp(help_target_from_command(&command));
    }

    shared_flags.into_invocation(command)
}

fn parse_alias(
    invocation: InvocationKind,
    args: &[String],
    shared_flags: SharedFlags,
) -> Result<ParsedInvocation> {
    let mut forwarded_args = normalize_forwarded_args(invocation, args.to_vec());
    let has_forwarded_args = !forwarded_args.is_empty();

    if has_forwarded_args {
        if shared_flags.help {
            forwarded_args.push("--help".to_string());
        }
        if shared_flags.version {
            forwarded_args.push("--version".to_string());
        }
    }

    let mut command = ParsedCommand::Execute {
        invocation,
        args: forwarded_args,
    };

    if !has_forwarded_args {
        if shared_flags.version {
            command = ParsedCommand::PrintVersions;
        } else if shared_flags.help {
            command = ParsedCommand::PrintHelp(help_topic_for_invocation(invocation));
        }
    }

    shared_flags.into_invocation(command)
}

fn parsed_hni_subcommand(subcommand: HniSubcommand, program: String) -> Result<ParsedCommand> {
    match subcommand {
        HniSubcommand::Install(args) => Ok(execute_from_args(InvocationKind::Ni, args)),
        HniSubcommand::Run(args) => Ok(execute_from_args(InvocationKind::Nr, args)),
        HniSubcommand::Exec(args) => Ok(execute_from_args(InvocationKind::Nlx, args)),
        HniSubcommand::Uninstall(args) => Ok(execute_from_args(InvocationKind::Nun, args)),
        HniSubcommand::Ci(args) => Ok(execute_from_args(InvocationKind::Nci, args)),
        HniSubcommand::Parallel(args) => Ok(execute_from_args(InvocationKind::Np, args)),
        HniSubcommand::Sequential(args) => Ok(execute_from_args(InvocationKind::Ns, args)),
        HniSubcommand::Help(args) => Ok(ParsedCommand::PrintHelp(help_target(args.command)?)),
        HniSubcommand::Doctor => Ok(ParsedCommand::Doctor),
        HniSubcommand::Completion(args) => Ok(ParsedCommand::Completion {
            shell: args.shell,
            program,
        }),
        HniSubcommand::Init(args) => Ok(ParsedCommand::Init { shell: args.shell }),
        HniSubcommand::Internal(args) => parse_internal_command(args),
    }
}

fn execute_from_args(invocation: InvocationKind, args: ForwardedArgs) -> ParsedCommand {
    ParsedCommand::Execute {
        invocation,
        args: normalize_forwarded_args(invocation, args.args),
    }
}

fn normalize_forwarded_args(invocation: InvocationKind, mut args: Vec<String>) -> Vec<String> {
    if matches!(
        invocation,
        InvocationKind::Ni | InvocationKind::Nun | InvocationKind::Nci
    ) && let Some(separator) = args.iter().position(|arg| arg == "--")
    {
        args.remove(separator);
    }

    args
}

fn parse_internal_command(args: InternalArgs) -> Result<ParsedCommand> {
    match args.command {
        Some(InternalSubcommand::RealNodePath) => Ok(ParsedCommand::InternalRealNodePath),
        Some(InternalSubcommand::ProfileLoop(args)) => Ok(ParsedCommand::InternalProfileLoop {
            invocation: internal_invocation(&args.invocation)?,
            args: args.args,
            iterations: args.iterations,
            timings: args.timings,
        }),
        None => Ok(ParsedCommand::PrintHelp(HelpTopic::Hni)),
    }
}

fn resolve_cwd(cwd_flags: &[PathBuf]) -> Result<PathBuf> {
    if cwd_flags.is_empty() {
        return env::current_dir().context("execution error: failed to read current directory");
    }

    let absolute_index = cwd_flags.iter().rposition(|segment| segment.is_absolute());
    let (mut cwd, start_index): (PathBuf, usize) = match absolute_index {
        Some(index) => (cwd_flags[index].clone(), index + 1),
        None => (
            env::current_dir().context("execution error: failed to read current directory")?,
            0,
        ),
    };

    for segment in &cwd_flags[start_index..] {
        cwd.push(segment);
    }

    Ok(cwd)
}

fn help_target(command: Option<String>) -> Result<HelpTopic> {
    let Some(command) = command else {
        return Ok(HelpTopic::Hni);
    };

    let normalized = command.to_ascii_lowercase();
    help_topic_by_name(&normalized)
        .ok_or_else(|| anyhow!("parse error: unknown help topic '{command}'. Try: hni help"))
}

fn help_target_from_command(command: &ParsedCommand) -> HelpTopic {
    match command {
        ParsedCommand::PrintHelp(topic) => *topic,
        ParsedCommand::Init { .. } => HelpTopic::Init,
        ParsedCommand::Execute { invocation, .. } => help_topic_for_invocation(*invocation),
        ParsedCommand::Doctor
        | ParsedCommand::Completion { .. }
        | ParsedCommand::InternalRealNodePath
        | ParsedCommand::InternalProfileLoop { .. }
        | ParsedCommand::PrintVersions => HelpTopic::Hni,
    }
}

pub fn hni_command() -> Command {
    HniPublicCli::command()
}

#[must_use]
pub fn command_parser(name: &'static str) -> Command {
    Command::new(name).arg(command_args_arg())
}

fn invocation_from_argv0(argv0: &str) -> InvocationKind {
    invocation_from_name(normalized_program_name(argv0).as_str()).unwrap_or(InvocationKind::Hni)
}

fn internal_invocation(name: &str) -> Result<InvocationKind> {
    invocation_from_name(name)
        .ok_or_else(|| anyhow!("parse error: unsupported internal invocation '{name}'"))
}

fn normalized_program_name(argv0: &str) -> String {
    let name = PathBuf::from(argv0)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(argv0)
        .to_ascii_lowercase();
    name.strip_suffix(".exe").unwrap_or(&name).to_string()
}

fn extract_shared_flags(args: &[String]) -> Result<(SharedFlags, Vec<String>)> {
    let mut flags = SharedFlags {
        cwd: Vec::new(),
        print_command: false,
        explain: false,
        fast_override: None,
        help: false,
        version: false,
    };
    let mut rest = Vec::new();
    let mut idx = 0;
    let mut passthrough = false;

    while idx < args.len() {
        let arg = &args[idx];
        if passthrough {
            rest.push(arg.clone());
            idx += 1;
            continue;
        }

        if arg == "--" {
            passthrough = true;
            rest.push(arg.clone());
            idx += 1;
            continue;
        }

        match arg.as_str() {
            "--print-command" => {
                flags.print_command = true;
                idx += 1;
            }
            "--explain" => {
                flags.explain = true;
                idx += 1;
            }
            "--fast" => {
                set_fast_override(&mut flags, true)?;
                idx += 1;
            }
            "--pm" => {
                set_fast_override(&mut flags, false)?;
                idx += 1;
            }
            "-h" | "--help" => {
                flags.help = true;
                idx += 1;
            }
            "-v" | "--version" => {
                flags.version = true;
                idx += 1;
            }
            "-C" | "--cwd" => {
                let Some(value) = args.get(idx + 1) else {
                    return Err(anyhow!("parse error: missing value for {arg}"));
                };
                flags.cwd.push(PathBuf::from(value));
                idx += 2;
            }
            _ if arg.starts_with("-C") && arg.len() > 2 => {
                flags.cwd.push(PathBuf::from(&arg[2..]));
                idx += 1;
            }
            _ if arg.starts_with("--cwd=") => {
                flags
                    .cwd
                    .push(PathBuf::from(arg.trim_start_matches("--cwd=")));
                idx += 1;
            }
            _ => {
                rest.push(arg.clone());
                idx += 1;
            }
        }
    }

    Ok((flags, rest))
}

fn set_fast_override(flags: &mut SharedFlags, value: bool) -> Result<()> {
    match flags.fast_override {
        Some(existing) if existing != value => {
            Err(anyhow!("parse error: --fast conflicts with --pm"))
        }
        _ => {
            flags.fast_override = Some(value);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn parse_args(argv: &[&str]) -> Result<ParsedInvocation> {
        let owned = strings(argv);
        let argv0 = owned
            .first()
            .ok_or_else(|| anyhow!("parse error: missing argv[0]"))?;
        let invocation = invocation_from_argv0(argv0);
        let (shared_flags, args) = extract_shared_flags(&owned[1..])?;

        if invocation == InvocationKind::Hni {
            parse_hni(argv0, &args, shared_flags)
        } else {
            parse_alias(invocation, &args, shared_flags)
        }
    }

    #[test]
    fn hni_fast_flag_sets_fast_override() {
        let parsed = parse_args(&["hni", "run", "--fast", "dev"]).unwrap();

        assert_eq!(parsed.fast_override, Some(true));
        match parsed.command {
            ParsedCommand::Execute { args, .. } => assert_eq!(args, vec!["dev"]),
            _ => panic!("expected execute command"),
        }
    }

    #[test]
    fn hni_print_command_after_positional_is_consumed_as_shared_flag() {
        let parsed = parse_args(&["hni", "install", "vite", "--print-command"]).unwrap();

        assert!(parsed.print_command);
        match parsed.command {
            ParsedCommand::Execute { args, .. } => assert_eq!(args, vec!["vite"]),
            _ => panic!("expected execute command"),
        }
    }

    #[test]
    fn hni_flags_after_passthrough_separator_are_forwarded() {
        let parsed = parse_args(&[
            "hni",
            "install",
            "vite",
            "--print-command",
            "--",
            "--version",
        ])
        .unwrap();

        assert!(parsed.print_command);
        match parsed.command {
            ParsedCommand::Execute { args, .. } => assert_eq!(args, vec!["vite", "--version"]),
            _ => panic!("expected execute command"),
        }
    }

    #[test]
    fn hni_extracts_short_and_long_cwd_flag_forms() {
        let parsed = parse_args(&["hni", "install", "-Ctmp", "--cwd=project", "vite"]).unwrap();

        assert_eq!(
            parsed.cwd,
            env::current_dir().unwrap().join("tmp").join("project")
        );
        match parsed.command {
            ParsedCommand::Execute { args, .. } => assert_eq!(args, vec!["vite"]),
            _ => panic!("expected execute command"),
        }
    }

    #[test]
    fn missing_cwd_value_is_parse_error() {
        let err = parse_args(&["hni", "install", "-C"]).unwrap_err();
        assert!(err.to_string().contains("parse error"));
    }

    #[test]
    fn conflicting_fast_and_pm_flags_are_rejected() {
        let err = parse_args(&["hni", "run", "--fast", "--pm", "dev"]).unwrap_err();
        assert!(err.to_string().contains("conflicts"));
    }

    #[test]
    fn alias_help_with_args_is_forwarded() {
        let parsed = parse_args(&["nlx", "vitest", "--help"]).unwrap();

        match parsed.command {
            ParsedCommand::Execute { args, .. } => {
                assert_eq!(args, vec!["vitest", "--help"]);
            }
            _ => panic!("expected execute command"),
        }
    }

    #[test]
    fn alias_help_without_args_prints_help() {
        let parsed = parse_args(&["nlx", "--help"]).unwrap();

        match parsed.command {
            ParsedCommand::PrintHelp(HelpTopic::Nlx) => {}
            _ => panic!("expected nlx help command"),
        }
    }
}
