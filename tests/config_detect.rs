use std::fs;

use hni::core::{
    config::HniConfig,
    detect::detect,
    types::{DetectionSource, PackageManager},
};

mod support;

#[test]
fn config_loads_and_env_overrides() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");
        fs::write(
            &cfg_path,
            "default_package_manager = \"pnpm\"\nglobal_package_manager = \"yarn\"\nfast_mode = true\n",
        )
        .unwrap();

        support::set_var("HNI_CONFIG_FILE", &cfg_path);
        support::set_var("HNI_GLOBAL_PACKAGE_MANAGER", "npm");
        support::set_var("HNI_FAST_MODE", "false");

        let cfg = HniConfig::load().unwrap();
        assert_eq!(cfg.default_package_manager, Some(PackageManager::Pnpm));
        assert_eq!(cfg.global_package_manager, PackageManager::Npm);
        assert!(!cfg.fast_mode);

        support::remove_var("HNI_CONFIG_FILE");
        support::remove_var("HNI_GLOBAL_PACKAGE_MANAGER");
        support::remove_var("HNI_FAST_MODE");
    });
}

#[test]
fn config_package_manager_values_are_case_and_whitespace_tolerant() {
    support::with_env_lock(|| {
        support::set_var("HNI_DEFAULT_PACKAGE_MANAGER", " Bun ");
        support::set_var("HNI_GLOBAL_PACKAGE_MANAGER", " PnPm ");

        let cfg = HniConfig::load().unwrap();

        support::remove_var("HNI_DEFAULT_PACKAGE_MANAGER");
        support::remove_var("HNI_GLOBAL_PACKAGE_MANAGER");

        assert_eq!(cfg.default_package_manager, Some(PackageManager::Bun));
        assert_eq!(cfg.global_package_manager, PackageManager::Pnpm);
    });
}

#[test]
fn explicit_config_path_must_exist() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing-config.toml");

        support::set_var("HNI_CONFIG_FILE", &missing);
        let err = HniConfig::load().unwrap_err();
        support::remove_var("HNI_CONFIG_FILE");

        let chain = err.chain().map(ToString::to_string).collect::<Vec<_>>();
        assert!(
            chain
                .iter()
                .any(|message| message.contains("config file not found"))
        );
        assert!(
            chain
                .iter()
                .any(|message| message.contains("failed to load"))
        );
    });
}

#[test]
fn detect_prefers_package_manager_field_over_lockfile() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'").unwrap();
    fs::write(
        dir.path().join("package.json"),
        r#"{"packageManager":"yarn@4.0.0"}"#,
    )
    .unwrap();

    let cfg = HniConfig::default();
    let detected = detect(dir.path(), &cfg).unwrap();

    assert_eq!(detected.agent, Some(PackageManager::YarnBerry));
    assert_eq!(detected.source, DetectionSource::PackageManagerField);
}

#[test]
fn detect_uses_config_fallback_when_no_lock_or_package_manager() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = HniConfig {
        default_package_manager: Some(PackageManager::Bun),
        ..HniConfig::default()
    };

    let detected = detect(dir.path(), &cfg).unwrap();
    assert_eq!(detected.agent, Some(PackageManager::Bun));
    assert_eq!(detected.source, DetectionSource::Config);
}
