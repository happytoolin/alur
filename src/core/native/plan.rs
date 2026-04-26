use std::fmt;

use crate::core::types::{NativeDenoTaskExecution, NativeLocalBinExecution, NativeScriptExecution};

pub(super) enum NativeDecision {
    Eligible(NativePlan),
    Ineligible(FallbackReason),
}

pub(super) enum NativePlan {
    Script(NativeScriptExecution),
    DenoTask(NativeDenoTaskExecution),
    LocalBin(NativeLocalBinExecution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FallbackReason {
    DenoScriptExecution,
    MissingNearestDenoProject,
    MissingNearestPackage,
    YarnBerryPnp,
    MissingScript(String),
    UnsupportedScriptEnv {
        event_name: String,
        pattern: &'static str,
    },
    MissingLocalBin,
    MissingLocalBinCommand,
    RemoteDenoExec,
}

impl fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DenoScriptExecution => write!(f, "deno script execution stays package-manager"),
            Self::MissingNearestDenoProject => {
                write!(f, "fast deno execution requires a nearest deno project")
            }
            Self::MissingNearestPackage => {
                write!(f, "fast script execution requires a nearest package.json")
            }
            Self::YarnBerryPnp => write!(
                f,
                "yarn berry Plug'n'Play does not expose node_modules/.bin; falling back to yarn execution"
            ),
            Self::MissingScript(script_name) => write!(
                f,
                "script '{script_name}' was not found in the nearest package.json"
            ),
            Self::UnsupportedScriptEnv {
                event_name,
                pattern,
            } => write!(
                f,
                "script '{event_name}' uses unsupported fast environment expansion ({pattern})"
            ),
            Self::MissingLocalBin => write!(
                f,
                "local binary not found in node_modules/.bin or package.json bin entries; falling back to package-manager exec"
            ),
            Self::MissingLocalBinCommand => {
                write!(f, "fast local bin execution requires a command")
            }
            Self::RemoteDenoExec => {
                write!(f, "remote deno exec stays in package-manager mode")
            }
        }
    }
}
