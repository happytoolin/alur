use hni::{
    core::types::{Intent, NodeShimMode},
    features::node_shim,
};

#[test]
fn routes_known_verbs() {
    let (decision, args) = node_shim::decide(&["run".into(), "dev".into()]);
    assert!(matches!(
        decision.mode,
        NodeShimMode::RouteToIntent(Intent::Run)
    ));
    assert_eq!(args, vec!["dev"]);
}

#[test]
fn passthroughs_unknown_verb() {
    let (decision, args) = node_shim::decide(&["script.js".into()]);
    assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    assert_eq!(args, vec!["script.js"]);
}

#[test]
fn passthroughs_with_double_dash() {
    let (decision, args) = node_shim::decide(&["--".into(), "-v".into()]);
    assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    assert_eq!(args, vec!["-v"]);
}

#[test]
fn routes_parallel_short_verb() {
    let (decision, args) = node_shim::decide(&["p".into(), "echo hi".into()]);
    assert!(matches!(decision.mode, NodeShimMode::RunParallel));
    assert_eq!(args, vec!["echo hi"]);
}

#[test]
fn routes_sequential_short_verb() {
    let (decision, args) = node_shim::decide(&["s".into(), "echo hi".into()]);
    assert!(matches!(decision.mode, NodeShimMode::RunSequential));
    assert_eq!(args, vec!["echo hi"]);
}

#[test]
fn passthroughs_flag_first_invocation() {
    let (decision, args) = node_shim::decide(&["-p".into(), "1+1".into()]);
    assert!(matches!(decision.mode, NodeShimMode::PassthroughNode));
    assert_eq!(args, vec!["-p", "1+1"]);
}
