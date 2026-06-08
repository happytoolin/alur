mod support;

use support::run_alur;

#[cfg(unix)]
use std::fs;

#[test]
fn explicit_missing_config_path_reports_config_error() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let missing = work.path().join("missing-config.toml");

        let output = run_alur(
            vec!["install", "vite"],
            &[("ALUR_CONFIG_FILE", missing.to_string_lossy().as_ref())],
        );
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("alur: config error:"));
        assert!(stderr.contains("config file not found"));
    });
}

#[test]
fn pre_execution_commands_do_not_load_config() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let missing = work.path().join("missing-config.toml");
        let missing = missing.to_string_lossy();

        let help = run_alur(vec!["help"], &[("ALUR_CONFIG_FILE", missing.as_ref())]);
        assert!(help.status.success(), "{help:?}");

        let completion = run_alur(
            vec!["completion", "bash"],
            &[("ALUR_CONFIG_FILE", missing.as_ref())],
        );
        assert!(completion.status.success(), "{completion:?}");

        let version = run_alur(vec!["--version"], &[("ALUR_CONFIG_FILE", missing.as_ref())]);
        assert!(version.status.success(), "{version:?}");
        assert!(String::from_utf8_lossy(&version.stdout).contains("alur v"));
    });
}

#[cfg(unix)]
#[test]
fn node_passthrough_does_not_load_config() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let missing = work.path().join("missing-config.toml");
        let real_node = work.path().join("real-node");

        fs::write(&real_node, "#!/bin/sh\nprintf 'v99.0.0\\n'\n").unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&real_node).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&real_node, perms).unwrap();
        }

        let output = support::run_alur_as(
            "node",
            vec!["--version"],
            &[
                ("ALUR_CONFIG_FILE", missing.to_string_lossy().as_ref()),
                ("ALUR_REAL_NODE", real_node.to_string_lossy().as_ref()),
            ],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "v99.0.0");
    });
}

#[cfg(unix)]
#[test]
fn original_node_shapes_keep_their_args_and_skip_config() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let missing = work.path().join("missing-config.toml");
        let real_node = work.path().join("real-node");

        fs::write(&real_node, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&real_node).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&real_node, perms).unwrap();
        }

        for args in [
            vec!["--run", "dev", "--print-command"],
            vec!["test", "--print-command", "--explain"],
        ] {
            let output = support::run_alur_as(
                "node",
                args.clone(),
                &[
                    ("ALUR_CONFIG_FILE", missing.to_string_lossy().as_ref()),
                    ("ALUR_REAL_NODE", real_node.to_string_lossy().as_ref()),
                ],
            );

            assert!(output.status.success(), "{output:?}");
            let stdout = String::from_utf8_lossy(&output.stdout);
            let actual = stdout.lines().collect::<Vec<_>>();
            assert_eq!(actual, args);
        }
    });
}

#[test]
fn unknown_help_topic_reports_parse_error() {
    support::with_env_lock(|| {
        let output = run_alur(
            vec!["help", "does-not-exist"],
            &[("ALUR_SKIP_PM_CHECK", "1")],
        );
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("alur: parse error:"));
        assert!(stderr.contains("unknown help topic"));
    });
}

#[test]
fn invalid_init_shell_reports_parse_error() {
    support::with_env_lock(|| {
        let output = run_alur(vec!["init", "tcsh"], &[("ALUR_SKIP_PM_CHECK", "1")]);
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("alur: parse error"));
        assert!(stderr.contains("tcsh"));
    });
}
