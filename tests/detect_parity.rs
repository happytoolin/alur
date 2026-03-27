use std::fs;

use hni::core::{
    config::HniConfig,
    detect::{DetectOptions, DetectStrategy, detect, detect_user_agent, detect_with_options},
    types::{DetectionSource, PackageManager},
};

mod support;

#[derive(Clone, Copy)]
struct DetectCase {
    name: &'static str,
    setup: fn(&std::path::Path),
    expected_agent: Option<PackageManager>,
    expected_version: Option<&'static str>,
    expected_source: DetectionSource,
}

#[test]
fn package_manager_field_cases_match_hni_semantics() {
    run_cases(
        &[
            DetectCase {
                name: "npm",
                setup: |dir| write_package_json(dir, r#"{"packageManager":"npm@7"}"#),
                expected_agent: Some(PackageManager::Npm),
                expected_version: Some("7"),
                expected_source: DetectionSource::PackageManagerField,
            },
            DetectCase {
                name: "pnpm-range",
                setup: |dir| write_package_json(dir, r#"{"packageManager":"^pnpm@8.0.0"}"#),
                expected_agent: Some(PackageManager::Pnpm),
                expected_version: Some("8.0.0"),
                expected_source: DetectionSource::PackageManagerField,
            },
            DetectCase {
                name: "pnpm-v6",
                setup: |dir| write_package_json(dir, r#"{"packageManager":"pnpm@6"}"#),
                expected_agent: Some(PackageManager::Pnpm),
                expected_version: Some("6"),
                expected_source: DetectionSource::PackageManagerField,
            },
            DetectCase {
                name: "yarn-berry",
                setup: |dir| write_package_json(dir, r#"{"packageManager":"yarn@3"}"#),
                expected_agent: Some(PackageManager::YarnBerry),
                expected_version: Some("3"),
                expected_source: DetectionSource::PackageManagerField,
            },
            DetectCase {
                name: "unknown-falls-through",
                setup: |dir| {
                    write_package_json(dir, r#"{"packageManager":"future-package-manager"}"#);
                    fs::write(dir.join("package-lock.json"), "lock").unwrap();
                },
                expected_agent: Some(PackageManager::Npm),
                expected_version: None,
                expected_source: DetectionSource::Lockfile,
            },
        ],
        None,
    );
}

#[test]
fn dev_engines_cases_match_hni_semantics() {
    run_cases(
        &[
            DetectCase {
                name: "npm",
                setup: |dir| {
                    write_package_json(
                        dir,
                        r#"{"devEngines":{"packageManager":{"name":"npm","version":"7"}}}"#,
                    )
                },
                expected_agent: Some(PackageManager::Npm),
                expected_version: Some("7"),
                expected_source: DetectionSource::DevEnginesField,
            },
            DetectCase {
                name: "pnpm-range",
                setup: |dir| {
                    write_package_json(
                        dir,
                        r#"{"devEngines":{"packageManager":{"name":"pnpm","version":"^8.0.0"}}}"#,
                    )
                },
                expected_agent: Some(PackageManager::Pnpm),
                expected_version: Some("8.0.0"),
                expected_source: DetectionSource::DevEnginesField,
            },
            DetectCase {
                name: "pnpm-v6",
                setup: |dir| {
                    write_package_json(
                        dir,
                        r#"{"devEngines":{"packageManager":{"name":"pnpm","version":"6"}}}"#,
                    )
                },
                expected_agent: Some(PackageManager::Pnpm),
                expected_version: Some("6"),
                expected_source: DetectionSource::DevEnginesField,
            },
            DetectCase {
                name: "yarn-berry",
                setup: |dir| {
                    write_package_json(
                        dir,
                        r#"{"devEngines":{"packageManager":{"name":"yarn","version":"4"}}}"#,
                    )
                },
                expected_agent: Some(PackageManager::YarnBerry),
                expected_version: Some("4"),
                expected_source: DetectionSource::DevEnginesField,
            },
            DetectCase {
                name: "unknown-falls-through",
                setup: |dir| {
                    write_package_json(
                        dir,
                        r#"{"devEngines":{"packageManager":{"name":"future-package-manager","version":"1.0.0"}}}"#,
                    );
                    fs::write(dir.join("package-lock.json"), "lock").unwrap();
                },
                expected_agent: Some(PackageManager::Npm),
                expected_version: None,
                expected_source: DetectionSource::Lockfile,
            },
        ],
        Some(DetectOptions {
            strategies: vec![
                DetectStrategy::DevEnginesField,
                DetectStrategy::Lockfile,
                DetectStrategy::PackageManagerField,
                DetectStrategy::InstallMetadata,
            ],
            stop_at: None,
        }),
    );
}

#[test]
fn install_metadata_cases_match_hni_semantics() {
    run_cases(
        &[
            DetectCase {
                name: "npm",
                setup: |dir| write_file(dir, "node_modules/.package-lock.json", "{}"),
                expected_agent: Some(PackageManager::Npm),
                expected_version: None,
                expected_source: DetectionSource::InstallMetadata,
            },
            DetectCase {
                name: "pnpm",
                setup: |dir| write_dir(dir, "node_modules/.pnpm"),
                expected_agent: Some(PackageManager::Pnpm),
                expected_version: None,
                expected_source: DetectionSource::InstallMetadata,
            },
            DetectCase {
                name: "yarn-classic",
                setup: |dir| write_file(dir, "node_modules/.yarn_integrity", ""),
                expected_agent: Some(PackageManager::Yarn),
                expected_version: None,
                expected_source: DetectionSource::InstallMetadata,
            },
            DetectCase {
                name: "yarn-berry-pnp",
                setup: |dir| write_file(dir, ".pnp.cjs", "module.exports = {};\n"),
                expected_agent: Some(PackageManager::YarnBerry),
                expected_version: None,
                expected_source: DetectionSource::InstallMetadata,
            },
            DetectCase {
                name: "deno",
                setup: |dir| write_dir(dir, "node_modules/.deno"),
                expected_agent: Some(PackageManager::Deno),
                expected_version: None,
                expected_source: DetectionSource::InstallMetadata,
            },
            DetectCase {
                name: "bun",
                setup: |dir| write_file(dir, "bun.lockb", "lock"),
                expected_agent: Some(PackageManager::Bun),
                expected_version: None,
                expected_source: DetectionSource::InstallMetadata,
            },
        ],
        Some(DetectOptions {
            strategies: vec![
                DetectStrategy::InstallMetadata,
                DetectStrategy::Lockfile,
                DetectStrategy::PackageManagerField,
                DetectStrategy::DevEnginesField,
            ],
            stop_at: None,
        }),
    );
}

#[test]
fn lockfile_cases_match_hni_semantics() {
    run_cases(
        &[
            DetectCase {
                name: "npm",
                setup: |dir| write_file(dir, "package-lock.json", "lock"),
                expected_agent: Some(PackageManager::Npm),
                expected_version: None,
                expected_source: DetectionSource::Lockfile,
            },
            DetectCase {
                name: "pnpm-workspace",
                setup: |dir| write_file(dir, "pnpm-workspace.yaml", "packages:\n  - packages/*\n"),
                expected_agent: Some(PackageManager::Pnpm),
                expected_version: None,
                expected_source: DetectionSource::Lockfile,
            },
            DetectCase {
                name: "yarn",
                setup: |dir| write_file(dir, "yarn.lock", "lock"),
                expected_agent: Some(PackageManager::Yarn),
                expected_version: None,
                expected_source: DetectionSource::Lockfile,
            },
            DetectCase {
                name: "deno",
                setup: |dir| write_file(dir, "deno.lock", "{}"),
                expected_agent: Some(PackageManager::Deno),
                expected_version: None,
                expected_source: DetectionSource::Lockfile,
            },
        ],
        None,
    );
}

#[test]
fn stop_at_limits_ancestor_detection() {
    let root = tempfile::tempdir().unwrap();
    let stop = root.path().join("mid");
    let nested = stop.join("deep");
    fs::create_dir_all(&nested).unwrap();
    fs::write(root.path().join("package-lock.json"), "lock").unwrap();

    let detected = detect_with_options(
        &nested,
        &HniConfig::default(),
        &DetectOptions {
            stop_at: Some(stop.clone()),
            ..DetectOptions::default()
        },
    )
    .unwrap();

    assert_ne!(detected.source, DetectionSource::Lockfile);
}

#[test]
fn package_manager_and_ancestor_lockfile_still_report_has_lock() {
    let root = tempfile::tempdir().unwrap();
    let pkg = root.path().join("packages").join("app");
    fs::create_dir_all(&pkg).unwrap();
    fs::write(root.path().join("pnpm-lock.yaml"), "lock").unwrap();
    write_package_json(&pkg, r#"{"packageManager":"npm@10.0.0"}"#);

    let detected = detect(&pkg, &HniConfig::default()).unwrap();
    assert_eq!(detected.agent, Some(PackageManager::Npm));
    assert_eq!(detected.source, DetectionSource::PackageManagerField);
    assert!(detected.has_lock);
}

#[test]
fn user_agent_detection_matches_supported_managers() {
    support::with_env_lock(|| {
        for (raw, expected) in [
            (
                "npm/10.9.0 node/v20.17.0 linux x64",
                Some(PackageManager::Npm),
            ),
            (
                "yarn/1.22.11 npm/? node/v14.17.6 darwin x64",
                Some(PackageManager::Yarn),
            ),
            (
                "pnpm/9.12.1 npm/? node/v20.17.0 linux x64",
                Some(PackageManager::Pnpm),
            ),
            (
                "bun/1.1.8 npm/? node/v21.6.0 linux x64",
                Some(PackageManager::Bun),
            ),
            (
                "deno/2.0.5 npm/? node/v21.6.0 linux x64",
                Some(PackageManager::Deno),
            ),
            ("unknown/1.0.0", None),
        ] {
            support::set_var("npm_config_user_agent", raw);
            assert_eq!(detect_user_agent(), expected, "user-agent {raw}");
        }
        support::remove_var("npm_config_user_agent");
    });
}

fn run_cases(cases: &[DetectCase], options: Option<DetectOptions>) {
    let config = HniConfig::default();
    for case in cases {
        let dir = tempfile::tempdir().unwrap();
        (case.setup)(dir.path());

        let detected = match &options {
            Some(options) => detect_with_options(dir.path(), &config, options).unwrap(),
            None => detect(dir.path(), &config).unwrap(),
        };

        assert_eq!(detected.agent, case.expected_agent, "case {}", case.name);
        assert_eq!(
            detected.version_hint.as_deref(),
            case.expected_version,
            "case {}",
            case.name
        );
        assert_eq!(detected.source, case.expected_source, "case {}", case.name);
    }
}

fn write_package_json(dir: &std::path::Path, raw: &str) {
    fs::write(dir.join("package.json"), raw).unwrap();
}

fn write_file(dir: &std::path::Path, relative: &str, raw: &str) {
    let path = dir.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, raw).unwrap();
}

fn write_dir(dir: &std::path::Path, relative: &str) {
    fs::create_dir_all(dir.join(relative)).unwrap();
}
