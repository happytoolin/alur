use anyhow::Result;

use crate::core::{
    batch,
    resolve::{self, ResolveContext},
    types::{BatchMode, Intent, NodeShimMode, ResolvedExecution},
};

pub fn handle(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    let (mode, routed_args) = decide(&args);

    let resolved = match mode {
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

pub fn decide(args: &[String]) -> (NodeShimMode, Vec<String>) {
    let Some((first, rest)) = args.split_first() else {
        return (NodeShimMode::PassthroughNode, Vec::new());
    };

    match routed_mode_for_verb(first) {
        Some(mode) => (mode, rest.to_vec()),
        None => (NodeShimMode::PassthroughNode, args.to_vec()),
    }
}

#[must_use]
pub fn is_routed_verb(value: &str) -> bool {
    routed_mode_for_verb(value).is_some()
}

fn routed_mode_for_verb(value: &str) -> Option<NodeShimMode> {
    match value {
        "p" => Some(NodeShimMode::RunParallel),
        "s" => Some(NodeShimMode::RunSequential),
        "install" | "i" => Some(NodeShimMode::RouteToIntent(Intent::Install)),
        "add" => Some(NodeShimMode::RouteToIntent(Intent::Add)),
        "uninstall" | "remove" => Some(NodeShimMode::RouteToIntent(Intent::Uninstall)),
        "run" => Some(NodeShimMode::RouteToIntent(Intent::Run)),
        "exec" | "x" | "dlx" => Some(NodeShimMode::RouteToIntent(Intent::Execute)),
        "ci" => Some(NodeShimMode::RouteToIntent(Intent::CleanInstall)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_install() {
        let (mode, args) = decide(&["install".into(), "vite".into()]);
        assert_eq!(args, vec!["vite"]);
        assert!(matches!(mode, NodeShimMode::RouteToIntent(Intent::Install)));
    }

    #[test]
    fn passthrough_unknown() {
        let (mode, args) = decide(&["server.js".into()]);
        assert_eq!(args, vec!["server.js"]);
        assert!(matches!(mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn passthrough_double_dash() {
        let (mode, args) = decide(&["--".into(), "-v".into()]);
        assert_eq!(args, vec!["--", "-v"]);
        assert!(matches!(mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn routes_parallel_short_verb() {
        let (mode, args) = decide(&["p".into(), "echo hi".into()]);
        assert_eq!(args, vec!["echo hi"]);
        assert!(matches!(mode, NodeShimMode::RunParallel));
    }

    #[test]
    fn routes_sequential_short_verb() {
        let (mode, args) = decide(&["s".into(), "echo hi".into()]);
        assert_eq!(args, vec!["echo hi"]);
        assert!(matches!(mode, NodeShimMode::RunSequential));
    }

    #[test]
    fn passthrough_flag_first() {
        let (mode, args) = decide(&["-p".into(), "1+1".into()]);
        assert_eq!(args, vec!["-p", "1+1"]);
        assert!(matches!(mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn passthrough_node_builtin_run_flag() {
        let (mode, args) = decide(&["--run".into(), "dev".into()]);
        assert_eq!(args, vec!["--run", "dev"]);
        assert!(matches!(mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn routes_exec_aliases() {
        for verb in ["exec", "x", "dlx"] {
            let (mode, args) = decide(&[verb.into(), "vitest".into()]);
            assert_eq!(args, vec!["vitest"]);
            assert!(matches!(mode, NodeShimMode::RouteToIntent(Intent::Execute)));
        }
    }

    #[test]
    fn routes_uninstall_aliases() {
        for verb in ["uninstall", "remove"] {
            let (mode, args) = decide(&[verb.into(), "lodash".into()]);
            assert_eq!(args, vec!["lodash"]);
            assert!(matches!(
                mode,
                NodeShimMode::RouteToIntent(Intent::Uninstall)
            ));
        }
    }

    #[test]
    fn routes_ci_to_clean_install() {
        let (mode, args) = decide(&["ci".into()]);
        assert_eq!(args, Vec::<String>::new());
        assert!(matches!(
            mode,
            NodeShimMode::RouteToIntent(Intent::CleanInstall)
        ));
    }

    #[test]
    fn passthrough_uppercase_verbs() {
        let (mode, args) = decide(&["RUN".into(), "dev".into()]);
        assert_eq!(args, vec!["RUN", "dev"]);
        assert!(matches!(mode, NodeShimMode::PassthroughNode));
    }

    #[test]
    fn routes_install_aliases() {
        for verb in ["install", "i", "add"] {
            let (mode, args) = decide(&[verb.into(), "vite".into()]);
            assert_eq!(args, vec!["vite"]);
            assert!(matches!(
                mode,
                NodeShimMode::RouteToIntent(Intent::Install | Intent::Add)
            ));
        }
    }

    #[test]
    fn passthrough_when_no_args() {
        let (mode, args) = decide(&[]);
        assert_eq!(args, Vec::<String>::new());
        assert!(matches!(mode, NodeShimMode::PassthroughNode));
    }
}
