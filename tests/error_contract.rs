mod support;

use support::run_hni;

#[test]
fn explicit_missing_config_path_reports_config_error() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let missing = work.path().join("missing-config.toml");

        let output = run_hni(
            vec!["install", "vite"],
            &[("HNI_CONFIG_FILE", missing.to_string_lossy().as_ref())],
        );
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("hni: config error:"));
        assert!(stderr.contains("config file not found"));
    });
}

#[test]
fn pre_execution_commands_do_not_load_config() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let missing = work.path().join("missing-config.toml");
        let missing = missing.to_string_lossy();

        let help = run_hni(vec!["help"], &[("HNI_CONFIG_FILE", missing.as_ref())]);
        assert!(help.status.success(), "{help:?}");

        let completion = run_hni(
            vec!["completion", "bash"],
            &[("HNI_CONFIG_FILE", missing.as_ref())],
        );
        assert!(completion.status.success(), "{completion:?}");
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
