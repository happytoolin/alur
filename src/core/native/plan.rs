use crate::core::types::{
    NativeDenoTaskExecution, NativeLocalBinExecution, NativeScriptExecution,
    NativeWorkspaceLocalBinExecution, NativeWorkspaceScriptExecution,
};
use thiserror::Error;

pub(super) enum NativeDecision {
    Eligible(NativePlan),
    Ineligible(FallbackReason),
}

pub(super) enum NativePlan {
    Script(NativeScriptExecution),
    DenoTask(NativeDenoTaskExecution),
    LocalBin(NativeLocalBinExecution),
    WorkspaceScripts(NativeWorkspaceScriptExecution),
    WorkspaceLocalBins(NativeWorkspaceLocalBinExecution),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum FallbackReason {
    #[error("{0}")]
    DenoTask(String),
    #[error("fast deno execution requires a nearest deno project")]
    MissingNearestDenoProject,
    #[error("fast script execution requires a nearest package.json")]
    MissingNearestPackage,
    #[error("yarn berry Plug'n'Play scripts require yarn execution")]
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
    #[error("{0}")]
    Workspace(String),
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
                "yarn berry Plug'n'Play scripts require yarn execution",
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
            (
                FallbackReason::Workspace(
                    "workspace fast mode requires a workspace root".to_string(),
                ),
                "workspace fast mode requires a workspace root",
            ),
        ];

        for (reason, expected) in cases {
            assert_eq!(reason.to_string(), expected);
        }
    }
}
