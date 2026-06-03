use std::fs;

use hni::{
    core::{runner, types::ResolvedExecution},
    platform::node::REAL_NODE_ENV,
};

mod support;

#[test]
fn passthrough_uses_explicit_real_node_env() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let real_node = dir.path().join(if cfg!(windows) {
            "real-node.exe"
        } else {
            "real-node"
        });
        fs::write(&real_node, "#!/bin/sh\nexit 0\n").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&real_node).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&real_node, perms).unwrap();
        }

        support::set_var(REAL_NODE_ENV, &real_node);

        let exec =
            ResolvedExecution::external("node", vec!["-v".into()], dir.path().to_path_buf(), true);

        let rendered = runner::format_command(&exec).unwrap();
        assert!(rendered.contains(real_node.to_string_lossy().as_ref()));

        support::remove_var(REAL_NODE_ENV);
    });
}
