use anyhow::Result;
use clap::Command;

use crate::{
    core::{resolve::ResolveContext, types::InvocationKind},
    features::node_shim,
};

pub use crate::core::types::HelpTopic;

use super::{
    commands,
    help::{command_help, init_help},
};

pub type CommandHandler =
    fn(Vec<String>, &ResolveContext) -> Result<Option<crate::core::types::ResolvedExecution>>;

#[derive(Clone, Copy)]
pub struct CommandSpec {
    pub name: &'static str,
    pub invocation: InvocationKind,
    pub help_topic: HelpTopic,
    pub about: &'static str,
    pub long_about: &'static str,
    pub examples: &'static str,
    pub handler: CommandHandler,
}

const COMMAND_SPECS: &[CommandSpec] = &[
    CommandSpec {
        name: "ni",
        invocation: InvocationKind::Ni,
        help_topic: HelpTopic::Ni,
        about: "install or add dependencies",
        long_about: "Routes installs to the package manager detected from packageManager or lockfile.",
        examples: "Examples:\n\
             \n\
             ni                   Install dependencies\n\
             ni vite              Add dependency\n\
             ni -D vitest         Add dev dependency\n\
             ni --frozen          Use lockfile-only install (nci behavior)\n\
             ni -- --help         Forward --help to underlying package manager\n\
             ni -g npm-check-updates",
        handler: commands::handle_ni,
    },
    CommandSpec {
        name: "nr",
        invocation: InvocationKind::Nr,
        help_topic: HelpTopic::Nr,
        about: "run package scripts",
        long_about: "Runs scripts in fast mode by default, then falls back to node or the detected package manager when needed.",
        examples: "Examples:\n\
             \n\
             nr                   Run 'start'\n\
             nr dev               Run dev script\n\
             nr --fast dev        Force fast mode\n\
             nr --pm dev          Force package-manager mode\n\
             nr test -- --watch   Pass extra args to script\n\
             nr --if-present lint Skip failure if script is missing",
        handler: commands::handle_nr,
    },
    CommandSpec {
        name: "nlx",
        invocation: InvocationKind::Nlx,
        help_topic: HelpTopic::Nlx,
        about: "execute package binaries",
        long_about: "Runs local or declared package binaries directly by default, then falls back to package-manager exec when needed.",
        examples: "Examples:\n\
             \n\
             nlx --fast eslint .\n\
             nlx vite@latest\n\
             nlx eslint .\n\
             nlx degit user/repo app",
        handler: commands::handle_nlx,
    },
    CommandSpec {
        name: "nun",
        invocation: InvocationKind::Nun,
        help_topic: HelpTopic::Nun,
        about: "uninstall dependencies",
        long_about: "Removes dependencies using the package manager detected from packageManager or lockfile.",
        examples: "Examples:\n\
             \n\
             nun lodash\n\
             nun react react-dom\n\
             nun -g typescript",
        handler: commands::handle_nun,
    },
    CommandSpec {
        name: "nci",
        invocation: InvocationKind::Nci,
        help_topic: HelpTopic::Nci,
        about: "clean install",
        long_about: "Performs lockfile-clean install when lockfile exists; falls back to install otherwise.",
        examples: "Examples:\n\
             \n\
             nci\n\
             nci --prefer-offline",
        handler: commands::handle_nci,
    },
    CommandSpec {
        name: "np",
        invocation: InvocationKind::Np,
        help_topic: HelpTopic::Np,
        about: "run shell commands in parallel",
        long_about: "Runs each argument as a separate shell command concurrently. Returns first non-zero code.",
        examples: "Examples:\n\
             \n\
             np \"npm:test\" \"npm:lint\"\n\
             np \"echo one\" \"echo two\"",
        handler: commands::handle_np,
    },
    CommandSpec {
        name: "ns",
        invocation: InvocationKind::Ns,
        help_topic: HelpTopic::Ns,
        about: "run shell commands sequentially",
        long_about: "Runs each argument in order and stops at first failure.",
        examples: "Examples:\n\
             \n\
             ns \"npm run build\" \"npm run test\"\n\
             ns \"echo pre\" \"echo post\"",
        handler: commands::handle_ns,
    },
    CommandSpec {
        name: "node",
        invocation: InvocationKind::NodeShim,
        help_topic: HelpTopic::Node,
        about: "package-manager-aware node shim",
        long_about: "Interprets known alur shim verbs and aliases, then routes them through alur command resolution.\n\
                     Every other invocation passes through to the real Node.js binary without argument parsing.",
        examples: "Passthrough examples:\n\
                     \n\
                     node script.js\n\
                     node -v\n\
                     node --run dev\n\
                     node -- --trace-warnings\n\
                     \n\
                     Routed examples:\n\
                     \n\
                     node install vite\n\
                     node add react\n\
                     node uninstall lodash\n\
                     node remove lodash\n\
                     node run dev -- --port=3000\n\
                     node x eslint .\n\
                     node p \"echo one\" \"echo two\"\n\
                     node s \"echo one\" \"echo two\"\n\
                     \n\
                     Routed verbs: p, s, install|i, add, uninstall|remove, run, exec|x|dlx, ci",
        handler: node_shim::handle,
    },
];

#[must_use]
pub fn command_specs() -> &'static [CommandSpec] {
    COMMAND_SPECS
}

#[must_use]
pub fn command_spec_by_name(name: &str) -> Option<&'static CommandSpec> {
    command_specs().iter().find(|spec| spec.name == name)
}

#[must_use]
pub fn command_spec_by_invocation(invocation: InvocationKind) -> Option<&'static CommandSpec> {
    command_specs()
        .iter()
        .find(|spec| spec.invocation == invocation)
}

#[must_use]
pub fn help_topic_by_name(name: &str) -> Option<HelpTopic> {
    match name {
        "alur" | "doctor" | "completion" | "help" => Some(HelpTopic::Alur),
        "init" => Some(HelpTopic::Init),
        "install" => Some(HelpTopic::Ni),
        "run" => Some(HelpTopic::Nr),
        "exec" => Some(HelpTopic::Nlx),
        "uninstall" => Some(HelpTopic::Nun),
        "ci" => Some(HelpTopic::Nci),
        "parallel" => Some(HelpTopic::Np),
        "sequential" => Some(HelpTopic::Ns),
        _ => command_spec_by_name(name).map(|spec| spec.help_topic),
    }
}

#[must_use]
pub fn help_topic_for_invocation(invocation: InvocationKind) -> HelpTopic {
    command_spec_by_invocation(invocation).map_or(HelpTopic::Alur, |spec| spec.help_topic)
}

#[must_use]
pub fn invocation_from_name(name: &str) -> Option<InvocationKind> {
    command_spec_by_name(name).map(|spec| spec.invocation)
}

#[must_use]
pub fn help_command_for_topic(topic: HelpTopic) -> Command {
    match topic {
        HelpTopic::Alur => super::help::top_level_help(),
        HelpTopic::Init => init_help(),
        _ => {
            let Some(spec) = command_specs().iter().find(|spec| spec.help_topic == topic) else {
                return super::help::top_level_help();
            };
            command_help(spec)
        }
    }
}
