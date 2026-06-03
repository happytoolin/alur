use crate::core::types::{NativeDenoTaskExecution, NativeLocalBinExecution, NativeScriptExecution};
use thiserror::Error;

pub(super) enum NativeDecision {
    Eligible(NativePlan),
    Ineligible(FallbackReason),
}

pub(super) enum NativePlan {
    Script(NativeScriptExecution),
    DenoTask(NativeDenoTaskExecution),
    LocalBin(NativeLocalBinExecution),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum FallbackReason {
    #[error("{0}")]
    DenoTask(String),
    #[error("fast deno execution requires a nearest deno project")]
    MissingNearestDenoProject,
    #[error("fast script execution requires a nearest package.json")]
    MissingNearestPackage,
    #[error(
        "yarn berry Plug'n'Play does not expose node_modules/.bin; falling back to yarn execution"
    )]
    YarnBerryPnp,
    #[error("script '{0}' was not found in the nearest package.json")]
    MissingScript(String),
    #[error("script '{event_name}' uses unsupported fast environment expansion ({pattern})")]
    UnsupportedScriptEnv {
        event_name: String,
        pattern: &'static str,
    },
    #[error(
        "local binary not found in node_modules/.bin or package.json bin entries; falling back to package-manager exec"
    )]
    MissingLocalBin,
    #[error("fast local bin execution requires a command")]
    MissingLocalBinCommand,
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
