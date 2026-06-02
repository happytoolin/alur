mod support;

use support::run_hni;

#[test]
fn explicit_missing_config_path_reports_config_error() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let missing = work.path().join("missing-hnirc");

        let output = run_hni(
            vec!["ni", "vite"],
            &[("HNI_CONFIG_FILE", missing.to_string_lossy().as_ref())],
        );
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("hni: config error:"));
        assert!(stderr.contains("config file not found"));
    });
}

#[test]
fn unknown_help_topic_reports_parse_error() {
    support::with_env_lock(|| {
        let output = run_hni(
            vec!["help", "does-not-exist"],
            &[("HNI_SKIP_PM_CHECK", "1")],
        );
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("hni: parse error:"));
        assert!(stderr.contains("unknown help topic"));
    });
}

#[test]
fn invalid_init_shell_reports_parse_error() {
    support::with_env_lock(|| {
        let output = run_hni(vec!["init", "tcsh"], &[("HNI_SKIP_PM_CHECK", "1")]);
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("hni: parse error"));
        assert!(stderr.contains("tcsh"));
    });
}
