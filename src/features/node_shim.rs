use anyhow::Result;

use crate::core::{
    batch::{self, BatchMode},
    resolve::{self, ResolveContext},
    types::{Intent, NodeShimMode, ResolvedExecution},
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

    if first == "--" {
        return (NodeShimMode::PassthroughNode, rest.to_vec());
    }

    if first.starts_with('-') {
        return (NodeShimMode::PassthroughNode, args.to_vec());
    }

    let verb = first.to_ascii_lowercase();
    let routed_args = rest.to_vec();

    match verb.as_str() {
        "p" => (NodeShimMode::RunParallel, routed_args),
        "s" => (NodeShimMode::RunSequential, routed_args),
        "install" | "i" => route(Intent::Install, routed_args),
        "add" => route(Intent::Add, routed_args),
        "run" => route(Intent::Run, routed_args),
        "exec" | "x" | "dlx" => route(Intent::Execute, routed_args),
        "update" | "upgrade" => route(Intent::Upgrade, routed_args),
        "uninstall" | "remove" => route(Intent::Uninstall, routed_args),
        "ci" => route(Intent::CleanInstall, routed_args),
        _ => (NodeShimMode::PassthroughNode, args.to_vec()),
    }
}

fn route(intent: Intent, args: Vec<String>) -> (NodeShimMode, Vec<String>) {
    (NodeShimMode::RouteToIntent(intent), args)
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
        assert_eq!(args, vec!["-v"]);
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
    fn routes_exec_aliases() {
        for verb in ["exec", "x", "dlx"] {
            let (mode, args) = decide(&[verb.into(), "vitest".into()]);
            assert_eq!(args, vec!["vitest"]);
            assert!(matches!(mode, NodeShimMode::RouteToIntent(Intent::Execute)));
        }
    }

    #[test]
    fn routes_upgrade_aliases() {
        for verb in ["update", "upgrade"] {
            let (mode, args) = decide(&[verb.into(), "vite".into()]);
            assert_eq!(args, vec!["vite"]);
            assert!(matches!(mode, NodeShimMode::RouteToIntent(Intent::Upgrade)));
        }
    }

    #[test]
    fn routes_uninstall_aliases() {
        for verb in ["uninstall", "remove"] {
            let (mode, args) = decide(&[verb.into(), "vite".into()]);
            assert_eq!(args, vec!["vite"]);
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
    fn routes_verbs_case_insensitively() {
        let (mode, args) = decide(&["RUN".into(), "dev".into()]);
        assert_eq!(args, vec!["dev"]);
        assert!(matches!(mode, NodeShimMode::RouteToIntent(Intent::Run)));
    }

    #[test]
    fn passthrough_when_no_args() {
        let (mode, args) = decide(&[]);
        assert_eq!(args, Vec::<String>::new());
        assert!(matches!(mode, NodeShimMode::PassthroughNode));
    }
}
