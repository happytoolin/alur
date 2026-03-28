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
        let cfg_path = dir.path().join("nirc");
        fs::write(
            &cfg_path,
            "defaultPackageManager=pnpm\nglobalPackageManager=yarn\nfastMode=true\n",
        )
        .unwrap();

        support::set_var("HNI_CONFIG_FILE", &cfg_path);
        support::set_var("HNI_GLOBAL_PACKAGE_MANAGER", "npm");
        support::set_var("HNI_FAST", "false");

        let cfg = HniConfig::load().unwrap();
        assert_eq!(cfg.default_package_manager, Some(PackageManager::Pnpm));
        assert_eq!(cfg.global_package_manager, PackageManager::Npm);
        assert!(!cfg.fast_mode);

        support::remove_var("HNI_CONFIG_FILE");
        support::remove_var("HNI_GLOBAL_PACKAGE_MANAGER");
        support::remove_var("HNI_FAST");
    });
}

#[test]
fn explicit_config_path_must_exist() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing-hnirc");

        support::set_var("HNI_CONFIG_FILE", &missing);
        let err = HniConfig::load().unwrap_err();
        support::remove_var("HNI_CONFIG_FILE");

        assert!(err.to_string().contains("config file not found"));
        assert!(err.to_string().contains("failed to load"));
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

#[test]
fn ignores_legacy_ni_environment_variables() {
    support::with_env_lock(|| {
        support::set_var("NI_DEFAULT_AGENT", "pnpm");
        support::set_var("NI_GLOBAL_AGENT", "yarn");
        support::set_var("NI_USE_SFW", "true");
        support::set_var("NI_AUTO_INSTALL", "true");

        let cfg = support::with_var_removed("HNI_FAST", HniConfig::load).unwrap();

        support::remove_var("NI_DEFAULT_AGENT");
        support::remove_var("NI_GLOBAL_AGENT");
        support::remove_var("NI_USE_SFW");
        support::remove_var("NI_AUTO_INSTALL");

        assert_eq!(cfg.default_package_manager, None);
        assert_eq!(cfg.global_package_manager, PackageManager::Npm);
        assert!(cfg.fast_mode);
    });
}

#[test]
fn ignores_legacy_nirc_fallback() {
    support::with_env_lock(|| {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        fs::create_dir_all(&home).unwrap();
        fs::write(
            home.join(".nirc"),
            "defaultPackageManager=pnpm\nglobalPackageManager=yarn\nfastMode=false\n",
        )
        .unwrap();

        support::set_var("HOME", &home);
        let cfg = support::with_var_removed("HNI_FAST", HniConfig::load).unwrap();
        support::remove_var("HOME");

        assert_eq!(cfg.default_package_manager, None);
        assert_eq!(cfg.global_package_manager, PackageManager::Npm);
        assert!(cfg.fast_mode);
    });
}
