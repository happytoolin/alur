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
    DenoTask(String),
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
}

impl fmt::Display for FallbackReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DenoTask(reason) => write!(f, "{reason}"),
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FallbackReason;

    #[test]
    fn fallback_reason_display_strings_are_stable() {
        let cases = [
            (
                FallbackReason::DenoTask("cycle detected".to_string()),
                "cycle detected",
            ),
            (
                FallbackReason::MissingNearestDenoProject,
                "fast deno execution requires a nearest deno project",
            ),
            (
                FallbackReason::MissingNearestPackage,
                "fast script execution requires a nearest package.json",
            ),
            (
                FallbackReason::YarnBerryPnp,
                "yarn berry Plug'n'Play does not expose node_modules/.bin; falling back to yarn execution",
            ),
            (
                FallbackReason::MissingScript("build".to_string()),
                "script 'build' was not found in the nearest package.json",
            ),
            (
                FallbackReason::UnsupportedScriptEnv {
                    event_name: "build".to_string(),
                    pattern: "npm_package_",
                },
                "script 'build' uses unsupported fast environment expansion (npm_package_)",
            ),
            (
                FallbackReason::MissingLocalBin,
                "local binary not found in node_modules/.bin or package.json bin entries; falling back to package-manager exec",
            ),
            (
                FallbackReason::MissingLocalBinCommand,
                "fast local bin execution requires a command",
            ),
        ];

        for (reason, expected) in cases {
            assert_eq!(reason.to_string(), expected);
        }
    }
}
