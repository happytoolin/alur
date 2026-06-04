use std::fs;

use hni::core::{
    config::HniConfig,
    detect::{detect, detect_user_agent},
    types::{DetectionResult, DetectionSource, PackageManager},
};

mod support;

#[derive(Clone, Copy)]
struct FixtureExpectation {
    name: &'static str,
    agent: PackageManager,
    version: Option<&'static str>,
    source: DetectionSource,
    has_lock: bool,
}

#[test]
fn packager_fixtures_match_hni_semantics() {
    run_fixture_cases(
        "packager",
        &[
            FixtureExpectation {
                name: "bun",
                agent: PackageManager::Bun,
                version: Some("0"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "deno",
                agent: PackageManager::Deno,
                version: Some("2"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "npm",
                agent: PackageManager::Npm,
                version: Some("7"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm",
                agent: PackageManager::Pnpm,
                version: Some("8"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm-version-range",
                agent: PackageManager::Pnpm,
                version: Some("8.0.0"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm-corepack-hash",
                agent: PackageManager::Pnpm,
                version: Some("9.12.3"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm-prerelease-corepack-hash",
                agent: PackageManager::Pnpm,
                version: Some("9.12.3-alpha.1"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm@6",
                agent: PackageManager::Pnpm,
                version: Some("6"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn",
                agent: PackageManager::Yarn,
                version: Some("1"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn@berry",
                agent: PackageManager::YarnBerry,
                version: Some("3"),
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn-berry-keyword",
                agent: PackageManager::YarnBerry,
                version: None,
                source: DetectionSource::PackageManagerField,
                has_lock: false,
            },
        ],
    );
}

#[test]
fn dev_engines_fixtures_match_hni_semantics() {
    run_fixture_cases(
        "dev-engines",
        &[
            FixtureExpectation {
                name: "bun",
                agent: PackageManager::Bun,
                version: Some("0"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "deno",
                agent: PackageManager::Deno,
                version: Some("2"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "npm",
                agent: PackageManager::Npm,
                version: Some("7"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm",
                agent: PackageManager::Pnpm,
                version: Some("8"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm-version-range",
                agent: PackageManager::Pnpm,
                version: Some("8.0.0"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm-corepack-hash",
                agent: PackageManager::Pnpm,
                version: Some("9.12.3"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm-prerelease-corepack-hash",
                agent: PackageManager::Pnpm,
                version: Some("9.12.3-alpha.1"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "mixed-array",
                agent: PackageManager::Pnpm,
                version: Some("9.1.0"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm@6",
                agent: PackageManager::Pnpm,
                version: Some("6"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn",
                agent: PackageManager::Yarn,
                version: Some("1"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn@berry",
                agent: PackageManager::YarnBerry,
                version: Some("3"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn-berry-range",
                agent: PackageManager::YarnBerry,
                version: Some("4.0.0"),
                source: DetectionSource::DevEnginesField,
                has_lock: false,
            },
        ],
    );
}

#[test]
fn lockfile_fixtures_match_hni_semantics() {
    run_fixture_cases(
        "lockfile",
        &[
            FixtureExpectation {
                name: "bun",
                agent: PackageManager::Bun,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
            FixtureExpectation {
                name: "deno",
                agent: PackageManager::Deno,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
            FixtureExpectation {
                name: "npm",
                agent: PackageManager::Npm,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
            FixtureExpectation {
                name: "pnpm",
                agent: PackageManager::Pnpm,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
            FixtureExpectation {
                name: "pnpm@6",
                agent: PackageManager::Pnpm,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
            FixtureExpectation {
                name: "yarn",
                agent: PackageManager::Yarn,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
            FixtureExpectation {
                name: "yarn@berry",
                agent: PackageManager::Yarn,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
        ],
    );
}

#[test]
fn install_metadata_fixtures_match_hni_semantics() {
    run_fixture_cases(
        "install-metadata",
        &[
            FixtureExpectation {
                name: "bun",
                agent: PackageManager::Bun,
                version: None,
                source: DetectionSource::Lockfile,
                has_lock: true,
            },
            FixtureExpectation {
                name: "deno",
                agent: PackageManager::Deno,
                version: None,
                source: DetectionSource::InstallMetadata,
                has_lock: false,
            },
            FixtureExpectation {
                name: "npm",
                agent: PackageManager::Npm,
                version: None,
                source: DetectionSource::InstallMetadata,
                has_lock: false,
            },
            FixtureExpectation {
                name: "pnpm",
                agent: PackageManager::Pnpm,
                version: None,
                source: DetectionSource::InstallMetadata,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn",
                agent: PackageManager::Yarn,
                version: None,
                source: DetectionSource::InstallMetadata,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn@berry",
                agent: PackageManager::YarnBerry,
                version: None,
                source: DetectionSource::InstallMetadata,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn@berry_pnp-v2",
                agent: PackageManager::YarnBerry,
                version: None,
                source: DetectionSource::InstallMetadata,
                has_lock: false,
            },
            FixtureExpectation {
                name: "yarn@berry_pnp-v3",
                agent: PackageManager::YarnBerry,
                version: None,
                source: DetectionSource::InstallMetadata,
                has_lock: false,
            },
        ],
    );
}

#[test]
fn unknown_fixture_variants_fall_back_to_local_defaults() {
    for category in ["packager", "dev-engines", "lockfile", "install-metadata"] {
        let detected = detect_fixture(category, "unknown");
        assert_eq!(detected.version_hint, None, "fixture {category}/unknown");
        assert!(!detected.has_lock, "fixture {category}/unknown");

        if support::command_exists("npm") {
            assert_eq!(
                detected.agent,
                Some(PackageManager::Npm),
                "fixture {category}/unknown"
            );
            assert_eq!(detected.source, DetectionSource::Fallback);
        } else {
            assert_eq!(detected.agent, None, "fixture {category}/unknown");
            assert_eq!(detected.source, DetectionSource::None);
        }
    }
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

fn run_fixture_cases(category: &str, cases: &[FixtureExpectation]) {
    for case in cases {
        let detected = detect_fixture(category, case.name);
        assert_eq!(
            detected.agent,
            Some(case.agent),
            "fixture {category}/{}",
            case.name
        );
        assert_eq!(
            detected.version_hint.as_deref(),
            case.version,
            "fixture {category}/{}",
            case.name
        );
        assert_eq!(
            detected.source, case.source,
            "fixture {category}/{}",
            case.name
        );
        assert_eq!(
            detected.has_lock, case.has_lock,
            "fixture {category}/{}",
            case.name
        );
    }
}

fn detect_fixture(category: &str, name: &str) -> DetectionResult {
    let dir = tempfile::tempdir().unwrap();
    support::copy_fixture_into(category, name, dir.path());
    detect(dir.path(), &HniConfig::default()).unwrap()
}

fn write_package_json(dir: &std::path::Path, raw: &str) {
    fs::write(dir.join("package.json"), raw).unwrap();
}
