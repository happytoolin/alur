use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

mod support;

use alur::{
    app::{
        cli::alur_command,
        command_registry::{
            command_spec_by_name, command_specs, help_command_for_topic, help_topic_by_name,
            invocation_from_name,
        },
        commands::{handle_npar, handle_nseq},
    },
    core::{
        config::AlurConfig,
        resolve::ResolveContext,
        types::{BatchMode, ExecutionStrategy, HelpTopic, InvocationKind},
    },
};
use figment::{
    Figment,
    providers::{Format, Toml},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DistWorkspace {
    dist: DistConfig,
}

#[derive(Debug, Deserialize)]
struct DistConfig {
    #[serde(rename = "bin-aliases")]
    bin_aliases: BTreeMap<String, Vec<String>>,
}

#[test]
fn alur_canonical_subcommands_resolve_commands() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("package-lock.json"), "lock").unwrap();
        fs::write(project.join("package.json"), r#"{"name":"x"}"#).unwrap();

        let output = run_alur(
            vec![
                "install",
                "-C",
                project.to_str().unwrap(),
                "vite",
                "--explain",
            ],
            &[("ALUR_SKIP_PM_CHECK", "1")],
        );
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("alur explain"));
        assert!(stdout.contains("resolved:"));
        assert!(stdout.contains("npm i vite"));

        let uninstall = run_alur(
            vec![
                "uninstall",
                "-C",
                project.to_str().unwrap(),
                "lodash",
                "--print-command",
            ],
            &[("ALUR_SKIP_PM_CHECK", "1")],
        );
        assert!(uninstall.status.success());
        let uninstall_stdout = String::from_utf8_lossy(&uninstall.stdout);
        assert_eq!(uninstall_stdout.trim(), "npm uninstall lodash");
    });
}

#[test]
fn alur_rejects_multicall_alias_subcommands() {
    support::with_env_lock(|| {
        let output = run_alur(vec!["ni", "--help"], &[("ALUR_SKIP_PM_CHECK", "1")]);
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("alur: parse error"));
    });
}

#[test]
fn command_registry_exposes_expected_public_surface() {
    let names = command_specs()
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["ni", "nr", "nex", "nrm", "nci", "npar", "nseq", "node"]
    );

    assert_eq!(invocation_from_name("nr"), Some(InvocationKind::Nr));
    assert_eq!(invocation_from_name("nrm"), Some(InvocationKind::Nrm));
    for removed_alias in ["nlx", "nun", "np", "ns"] {
        assert_eq!(invocation_from_name(removed_alias), None);
    }
    assert_eq!(help_topic_by_name("completion"), Some(HelpTopic::Alur));
    assert_eq!(help_topic_by_name("install"), Some(HelpTopic::Ni));
    assert_eq!(help_topic_by_name("uninstall"), Some(HelpTopic::Nrm));
    assert_eq!(
        command_spec_by_name("init").map(|spec| spec.name),
        None,
        "init is a top-level alur command, not a multicall alias"
    );

    let alur_subcommand_names = alur_command()
        .get_subcommands()
        .filter(|command| !command.is_hide_set())
        .map(|command| command.get_name().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        alur_subcommand_names,
        vec![
            "install",
            "uninstall",
            "run",
            "exec",
            "ci",
            "parallel",
            "sequential",
            "help",
            "doctor",
            "completion",
            "init",
        ]
    );

    assert_eq!(help_command_for_topic(HelpTopic::Nr).get_name(), "nr");
    assert_eq!(help_command_for_topic(HelpTopic::Init).get_name(), "init");
}

#[test]
fn jsr_invocations_stay_in_sync_with_alias_manifest_and_command_registry() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let aliases_raw = fs::read_to_string(root.join("aliases.json")).unwrap();
    let aliases: serde_json::Value = serde_json::from_str(&aliases_raw).unwrap();
    let alias_names = aliases
        .get("alur")
        .and_then(serde_json::Value::as_array)
        .expect("aliases.json must define alur aliases")
        .iter()
        .map(|alias| {
            alias
                .as_str()
                .expect("alur aliases must be strings")
                .to_string()
        })
        .collect::<Vec<_>>();

    let registry_alias_names = command_specs()
        .iter()
        .filter(|spec| spec.name != "node")
        .map(|spec| spec.name.to_string())
        .collect::<Vec<_>>();
    assert_eq!(alias_names, registry_alias_names);

    let mut expected_invocations = vec!["alur".to_string()];
    expected_invocations.extend(alias_names);

    let shared = fs::read_to_string(root.join("jsr/shared.ts")).unwrap();
    assert_eq!(parse_jsr_invocations(&shared), expected_invocations);

    let mod_entrypoint = fs::read_to_string(root.join("jsr/mod.ts")).unwrap();
    assert!(
        mod_entrypoint.contains("INVOCATIONS"),
        "jsr/mod.ts must re-export INVOCATIONS"
    );

    let jsr_json_raw = fs::read_to_string(root.join("jsr.json")).unwrap();
    let jsr_json: serde_json::Value = serde_json::from_str(&jsr_json_raw).unwrap();
    let exports = jsr_json
        .get("exports")
        .and_then(serde_json::Value::as_object)
        .expect("jsr.json must define exports");
    let expected_export_keys = std::iter::once(".".to_string())
        .chain(expected_invocations.iter().map(|name| format!("./{name}")))
        .collect::<BTreeSet<_>>();
    let actual_export_keys = exports.keys().cloned().collect::<BTreeSet<_>>();
    assert_eq!(actual_export_keys, expected_export_keys);
    assert_eq!(
        exports.get(".").and_then(serde_json::Value::as_str),
        Some("./jsr/mod.ts")
    );

    for invocation in &expected_invocations {
        let export = format!("./{invocation}");
        let export_path = format!("./jsr/{invocation}.ts");
        assert_eq!(
            exports.get(&export).and_then(serde_json::Value::as_str),
            Some(export_path.as_str())
        );

        let launcher_path = root.join("jsr").join(format!("{invocation}.ts"));
        let launcher = fs::read_to_string(&launcher_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", launcher_path.display()));
        assert!(
            launcher.contains(&format!("runInvocation(\"{invocation}\")")),
            "{} must invoke the matching JSR command",
            launcher_path.display()
        );
    }

    let expected_launcher_names = expected_invocations
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual_launcher_names = fs::read_dir(root.join("jsr"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .filter(|name| name != "mod.ts" && name != "shared.ts")
        .map(|name| name.trim_end_matches(".ts").to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_launcher_names, expected_launcher_names);
}

#[test]
fn release_aliases_stay_in_sync_with_alias_manifest() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let aliases_raw = fs::read_to_string(root.join("aliases.json")).unwrap();
    let aliases: serde_json::Value = serde_json::from_str(&aliases_raw).unwrap();
    let alias_names = aliases
        .get("alur")
        .and_then(serde_json::Value::as_array)
        .expect("aliases.json must define alur aliases")
        .iter()
        .map(|alias| alias.as_str().expect("aliases must be strings").to_string())
        .collect::<Vec<_>>();

    let dist: DistWorkspace = Figment::new()
        .merge(Toml::file(root.join("dist-workspace.toml")))
        .extract()
        .unwrap();

    assert_eq!(
        dist.dist.bin_aliases.get("alur"),
        Some(&alias_names),
        "cargo-dist bin-aliases must not publish removed or missing multicall aliases"
    );
}

#[test]
fn removed_aliases_are_absent_from_public_surface_files() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    for relative in [
        "README.md",
        "docs/fast-compat.md",
        ".github/og-image.svg",
        "aliases.json",
        "jsr.json",
        "jsr/shared.ts",
        "dist-workspace.toml",
    ] {
        let path = root.join(relative);
        let content = fs::read_to_string(&path).unwrap();
        for alias in ["nru", "na", "nlx", "nun", "np", "ns"] {
            assert!(
                !contains_token(&content, alias),
                "{relative} still references removed alias {alias}"
            );
        }
    }
}

#[test]
fn alur_pre_execution_commands_are_available() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let doctor = run_alur(vec!["doctor"], &[("ALUR_SKIP_PM_CHECK", "1")]);
        assert!(doctor.status.success());
        let doctor_out = String::from_utf8_lossy(&doctor.stdout);
        assert!(doctor_out.contains("alur doctor"));

        let completion = run_alur(vec!["completion", "bash"], &[]);
        assert!(completion.status.success());
        let completion_out = String::from_utf8_lossy(&completion.stdout);
        assert!(completion_out.contains("alur"));
        assert!(!completion_out.contains(" internal"));
        assert!(!completion_out.contains(" ni"));

        let top_help = run_alur(vec!["help"], &[("ALUR_SKIP_PM_CHECK", "1")]);
        assert!(top_help.status.success());
        let top_help_out = String::from_utf8_lossy(&top_help.stdout);
        assert!(top_help_out.contains("Usage: alur"));

        let nr_help = run_alur(vec!["help", "nr"], &[("ALUR_SKIP_PM_CHECK", "1")]);
        assert!(nr_help.status.success());
        let nr_help_out = String::from_utf8_lossy(&nr_help.stdout);
        assert!(nr_help_out.contains("Usage: nr"));

        let version = run_alur(vec!["--version"], &[("ALUR_SKIP_PM_CHECK", "1")]);
        assert!(version.status.success());
        let version_out = String::from_utf8_lossy(&version.stdout);
        assert!(version_out.contains("alur v"));

        let init_home = work.path().join("init-home");
        let init_data = work.path().join("init-data");
        fs::create_dir_all(&init_home).unwrap();
        fs::create_dir_all(&init_data).unwrap();
        let init = run_alur(
            vec!["init", "bash"],
            &[
                ("ALUR_SKIP_PM_CHECK", "1"),
                ("HOME", init_home.to_string_lossy().as_ref()),
                ("XDG_DATA_HOME", init_data.to_string_lossy().as_ref()),
            ],
        );
        assert!(init.status.success());
        let init_out = String::from_utf8_lossy(&init.stdout);
        assert!(init_out.contains("# alur init"));
    });
}

#[test]
fn app_command_handlers_build_batch_executions() {
    let work = tempfile::tempdir().unwrap();
    let ctx = ResolveContext::with_package_manager_checks(
        work.path().to_path_buf(),
        AlurConfig::default(),
        false,
    );

    let parallel = handle_npar(vec!["echo one".to_string(), "echo two".to_string()], &ctx)
        .unwrap()
        .expect("npar should build a batch execution");
    assert!(matches!(
        parallel.strategy,
        ExecutionStrategy::InternalBatch {
            mode: BatchMode::Parallel,
            ..
        }
    ));
    assert_eq!(parallel.args, vec!["echo one", "echo two"]);

    let sequential = handle_nseq(vec!["echo one".to_string(), "echo two".to_string()], &ctx)
        .unwrap()
        .expect("nseq should build a batch execution");
    assert!(matches!(
        sequential.strategy,
        ExecutionStrategy::InternalBatch {
            mode: BatchMode::Sequential,
            ..
        }
    ));
    assert_eq!(sequential.args, vec!["echo one", "echo two"]);
}

fn run_alur(args: Vec<&str>, extra_env: &[(&str, &str)]) -> std::process::Output {
    support::run_alur(args, extra_env)
}

fn parse_jsr_invocations(shared: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut collecting = false;

    for line in shared.lines() {
        let line = line.trim();
        if line == "export const INVOCATIONS = [" {
            collecting = true;
            continue;
        }
        if collecting && line.starts_with("] as const") {
            break;
        }
        if collecting
            && let Some(rest) = line.strip_prefix('"')
            && let Some((name, _)) = rest.split_once('"')
        {
            names.push(name.to_string());
        }
    }

    names
}

fn contains_token(content: &str, token: &str) -> bool {
    content
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '@')
        .any(|candidate| candidate == token)
}
