use anyhow::Result;

use crate::{
    core::{
        batch::{self, BatchMode},
        resolve::{self, ResolveContext},
        types::{Intent, NodeShimDecision, NodeShimMode, ResolvedExecution},
    },
    platform::node::{NODE_SHIM_ENV, SHIM_ACTIVE_ENV},
};

pub fn handle(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    let (decision, routed_args) = decide(&args);

    let resolved = match decision.mode {
        NodeShimMode::PassthroughNode => resolve::resolve_node_passthrough(routed_args, ctx.cwd()),
        NodeShimMode::RouteToIntent(intent) => {
            resolve::resolve_node_routed(intent, routed_args, ctx)?
        }
        NodeShimMode::RunParallel => {
            batch::make_execution(BatchMode::Parallel, routed_args, ctx.cwd())
        }
        NodeShimMode::RunSequential => {
            batch::make_execution(BatchMode::Sequential, routed_args, ctx.cwd())
        }
    };

    Ok(Some(resolved))
}

pub fn decide(args: &[String]) -> (NodeShimDecision, Vec<String>) {
    let shim_active = std::env::var_os(SHIM_ACTIVE_ENV).is_some();
    let shim_disabled = std::env::var_os(NODE_SHIM_ENV)
        .and_then(|value| value.into_string().ok())
        .is_some_and(|value| node_shim_disabled(&value));
    decide_with_shim_state(args, shim_active, shim_disabled)
}

pub fn decide_with_shim_state(
    args: &[String],
    shim_active: bool,
    shim_disabled: bool,
) -> (NodeShimDecision, Vec<String>) {
    if shim_disabled {
        return passthrough(
            args.to_vec(),
            "node shim disabled by HNI_NODE environment override",
        );
    }

    if shim_active {
        return passthrough(args.to_vec(), "shim recursion guard active");
    }

    let Some((first, rest)) = args.split_first() else {
        return passthrough(Vec::new(), "node with no args should open REPL");
    };

    if first == "--" {
        return passthrough(
            rest.to_vec(),
            "double-dash explicitly requests raw node passthrough",
        );
    }

    if first.starts_with('-') {
        return passthrough(
            args.to_vec(),
            "flag-first invocation should passthrough to real node",
        );
    }

    let verb = first.to_ascii_lowercase();
    let routed_args = rest.to_vec();

    match verb.as_str() {
        "p" => decision(
            NodeShimMode::RunParallel,
            routed_args,
            "route p through batch parallel",
        ),
        "s" => decision(
            NodeShimMode::RunSequential,
            routed_args,
            "route s through batch sequential",
        ),
        "install" | "i" => route(
            Intent::Install,
            routed_args,
            "route install through ni parser",
        ),
        "add" => route(Intent::Add, routed_args, "route add to package manager add"),
        "run" => route(Intent::Run, routed_args, "route run through nr parser"),
        "exec" | "x" | "dlx" => route(Intent::Execute, routed_args, "route exec through nlx"),
        "update" | "upgrade" => route(Intent::Upgrade, routed_args, "route update through nru"),
        "uninstall" | "remove" => route(
            Intent::Uninstall,
            routed_args,
            "route uninstall through nun",
        ),
        "ci" => route(Intent::CleanInstall, routed_args, "route ci through nci"),
        _ => passthrough(args.to_vec(), "unknown verb: passthrough to real node"),
    }
}

fn route(intent: Intent, args: Vec<String>, reason: &str) -> (NodeShimDecision, Vec<String>) {
    decision(NodeShimMode::RouteToIntent(intent), args, reason)
}

fn passthrough(args: Vec<String>, reason: &str) -> (NodeShimDecision, Vec<String>) {
    decision(NodeShimMode::PassthroughNode, args, reason)
}

fn decision(
    mode: NodeShimMode,
    args: Vec<String>,
    reason: &str,
) -> (NodeShimDecision, Vec<String>) {
    (
        NodeShimDecision {
            mode,
            reason: reason.to_string(),
        },
        args,
    )
}

fn node_shim_disabled(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "0" | "off" | "false" | "disable" | "disabled"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_install() {
        let (decision, args) = decide(&["install".into(), "vite".into()]);
        assert_eq!(args, vec!["vite"]);
        assert!(matches!(
            decision.mode,
            NodeShimMode::RouteToIntent(Intent::Install)
        ));
    }

    #[test]
    fn passthrough_unknown() {
        let (decision, args) = decide(&["server.js".into()]);
        assert_eq!(args, vec!["server.js"]);
        assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn env_override_disables_shim() {
        let (decision, args) = decide_with_shim_state(&["run".into(), "dev".into()], false, true);
        assert_eq!(args, vec!["run", "dev"]);
        assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn passthrough_double_dash() {
        let (decision, args) = decide(&["--".into(), "-v".into()]);
        assert_eq!(args, vec!["-v"]);
        assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn routes_parallel_short_verb() {
        let (decision, args) = decide(&["p".into(), "echo hi".into()]);
        assert_eq!(args, vec!["echo hi"]);
        assert!(matches!(decision.mode, NodeShimMode::RunParallel));
    }

    #[test]
    fn routes_sequential_short_verb() {
        let (decision, args) = decide(&["s".into(), "echo hi".into()]);
        assert_eq!(args, vec!["echo hi"]);
        assert!(matches!(decision.mode, NodeShimMode::RunSequential));
    }

    #[test]
    fn passthrough_flag_first() {
        let (decision, args) = decide(&["-p".into(), "1+1".into()]);
        assert_eq!(args, vec!["-p", "1+1"]);
        assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn routes_exec_aliases() {
        for verb in ["exec", "x", "dlx"] {
            let (decision, args) = decide(&[verb.into(), "vitest".into()]);
            assert_eq!(args, vec!["vitest"]);
            assert!(matches!(
                decision.mode,
                NodeShimMode::RouteToIntent(Intent::Execute)
            ));
        }
    }

    #[test]
    fn routes_upgrade_aliases() {
        for verb in ["update", "upgrade"] {
            let (decision, args) = decide(&[verb.into(), "vite".into()]);
            assert_eq!(args, vec!["vite"]);
            assert!(matches!(
                decision.mode,
                NodeShimMode::RouteToIntent(Intent::Upgrade)
            ));
        }
    }

    #[test]
    fn routes_uninstall_aliases() {
        for verb in ["uninstall", "remove"] {
            let (decision, args) = decide(&[verb.into(), "vite".into()]);
            assert_eq!(args, vec!["vite"]);
            assert!(matches!(
                decision.mode,
                NodeShimMode::RouteToIntent(Intent::Uninstall)
            ));
        }
    }

    #[test]
    fn routes_ci_to_clean_install() {
        let (decision, args) = decide(&["ci".into()]);
        assert_eq!(args, Vec::<String>::new());
        assert!(matches!(
            decision.mode,
            NodeShimMode::RouteToIntent(Intent::CleanInstall)
        ));
    }

    #[test]
    fn routes_verbs_case_insensitively() {
        let (decision, args) = decide(&["RUN".into(), "dev".into()]);
        assert_eq!(args, vec!["dev"]);
        assert!(matches!(
            decision.mode,
            NodeShimMode::RouteToIntent(Intent::Run)
        ));
    }

    #[test]
    fn passthrough_when_no_args() {
        let (decision, args) = decide(&[]);
        assert_eq!(args, Vec::<String>::new());
        assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn disabled_values_are_recognized_case_insensitively() {
        for value in ["0", "off", "OFF", "false", "False", "disable", "disabled"] {
            assert!(node_shim_disabled(value), "{value} should disable the shim");
        }

        for value in ["1", "on", "true", "npm"] {
            assert!(
                !node_shim_disabled(value),
                "{value} should not disable the shim"
            );
        }
    }
}
