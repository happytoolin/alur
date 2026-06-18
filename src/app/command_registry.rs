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
        name: "nex",
        invocation: InvocationKind::Nex,
        help_topic: HelpTopic::Nex,
        about: "execute package binaries",
        long_about: "Runs local or declared package binaries directly by default, then falls back to package-manager exec when needed.",
        examples: "Examples:\n\
             \n\
             nex --fast eslint .\n\
             nex vite@latest\n\
             nex eslint .\n\
             nex degit user/repo app",
        handler: commands::handle_nex,
    },
    CommandSpec {
        name: "nrm",
        invocation: InvocationKind::Nrm,
        help_topic: HelpTopic::Nrm,
        about: "uninstall dependencies",
        long_about: "Removes dependencies using the package manager detected from packageManager or lockfile.",
        examples: "Examples:\n\
             \n\
             nrm lodash\n\
             nrm react react-dom\n\
             nrm -g typescript",
        handler: commands::handle_nrm,
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
        name: "npar",
        invocation: InvocationKind::Npar,
        help_topic: HelpTopic::Npar,
        about: "run shell commands in parallel",
        long_about: "Runs each argument as a separate shell command concurrently. Returns first non-zero code.",
        examples: "Examples:\n\
             \n\
             npar \"npm:test\" \"npm:lint\"\n\
             npar \"echo one\" \"echo two\"",
        handler: commands::handle_npar,
    },
    CommandSpec {
        name: "nseq",
        invocation: InvocationKind::Nseq,
        help_topic: HelpTopic::Nseq,
        about: "run shell commands sequentially",
        long_about: "Runs each argument in order and stops at first failure.",
        examples: "Examples:\n\
             \n\
             nseq \"npm run build\" \"npm run test\"\n\
             nseq \"echo pre\" \"echo post\"",
        handler: commands::handle_nseq,
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
        "exec" => Some(HelpTopic::Nex),
        "uninstall" => Some(HelpTopic::Nrm),
        "ci" => Some(HelpTopic::Nci),
        "parallel" => Some(HelpTopic::Npar),
        "sequential" => Some(HelpTopic::Nseq),
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
