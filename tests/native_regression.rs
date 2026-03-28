#![cfg(unix)]

use std::{
    fs,
    path::{Path, PathBuf},
};

mod support;

use support::{command_exists, run_command, run_hni_owned};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Manager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl Manager {
    fn command(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
        }
    }

    fn package_manager(self) -> &'static str {
        match self {
            Self::Npm => "npm@11.6.2",
            Self::Pnpm => "pnpm@10.28.1",
            Self::Yarn => "yarn@1.22.22",
            Self::Bun => "bun@1.3.5",
        }
    }

    fn lockfile(self) -> &'static str {
        match self {
            Self::Npm => "package-lock.json",
            Self::Pnpm => "pnpm-lock.yaml",
            Self::Yarn => "yarn.lock",
            Self::Bun => "bun.lockb",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum CommandFamily {
    Nr,
    Nlx,
}

#[derive(Clone, Copy, Debug)]
enum Classification {
    Equivalence,
    Fallback,
}

#[derive(Clone, Copy, Debug)]
struct NativeRegressionCase {
    name: &'static str,
    upstream_file: &'static str,
    upstream_test: &'static str,
    manager: Manager,
    family: CommandFamily,
    classification: Classification,
    setup: fn(&Path),
    subject: &'static str,
    forwarded_args: &'static [&'static str],
    fallback_reason_fragment: Option<&'static str>,
    assert_state: fn(&Path),
}

#[derive(Debug)]
struct CaseRun {
    _work: tempfile::TempDir,
    oracle_root: PathBuf,
    hni_root: PathBuf,
    oracle_output: std::process::Output,
    hni_output: std::process::Output,
    debug_output: std::process::Output,
    explain_output: Option<std::process::Output>,
}

#[test]
fn native_regression_cases_match_or_fallback_to_the_package_manager() {
    support::with_env_lock(|| {
        for case in native_regression_cases() {
            if !command_exists(case.manager.command()) {
                continue;
            }

            let run = run_case(case);
            assert!(
                exit_codes_match(&run.oracle_output, &run.hni_output),
                "native regression '{}' diverged in exit code from oracle\nupstream: {} :: {}\noracle status={:?}\nhni status={:?}\noracle stdout={}\nhni stdout={}\noracle stderr={}\nhni stderr={}",
                case.name,
                case.upstream_file,
                case.upstream_test,
                run.oracle_output.status.code(),
                run.hni_output.status.code(),
                String::from_utf8_lossy(&run.oracle_output.stdout),
                String::from_utf8_lossy(&run.hni_output.stdout),
                String::from_utf8_lossy(&run.oracle_output.stderr),
                String::from_utf8_lossy(&run.hni_output.stderr),
            );

            (case.assert_state)(&run.oracle_root);
            (case.assert_state)(&run.hni_root);

            match case.classification {
                Classification::Equivalence => {
                    let stdout = String::from_utf8_lossy(&run.debug_output.stdout);
                    assert!(
                        stdout.starts_with("hni fast:"),
                        "equivalence case '{}' did not resolve natively: {stdout}",
                        case.name,
                    );
                }
                Classification::Fallback => {
                    let explain = run
                        .explain_output
                        .expect("fallback cases should capture explain output");
                    let stdout = String::from_utf8_lossy(&explain.stdout);
                    assert!(
                        stdout.contains("fast_status: fallback"),
                        "fallback case '{}' did not report fallback: {stdout}",
                        case.name
                    );
                    if let Some(reason) = case.fallback_reason_fragment {
                        assert!(
                            stdout.contains(reason),
                            "fallback case '{}' missing reason fragment {:?}: {stdout}",
                            case.name,
                            reason
                        );
                    }
                }
            }
        }
    });
}

fn native_regression_cases() -> Vec<NativeRegressionCase> {
    vec![
        NativeRegressionCase {
            name: "npm-hooks-and-forwarded-args",
            upstream_file: "https://github.com/npm/run-script/blob/main/test/run-script-pkg.js",
            upstream_test: "stdio inherit args and no pkgid / run-script-pkg",
            manager: Manager::Npm,
            family: CommandFamily::Nr,
            classification: Classification::Equivalence,
            setup: setup_hooked_script_fixture_npm,
            subject: "dev",
            forwarded_args: &["alpha", "beta"],
            fallback_reason_fragment: None,
            assert_state: assert_hooked_script_fixture,
        },
        NativeRegressionCase {
            name: "npm-unsupported-env-expansion-falls-back",
            upstream_file: "https://github.com/npm/run-script/blob/main/lib/package-envs.js",
            upstream_test: "package env expansion remains package-manager-owned",
            manager: Manager::Npm,
            family: CommandFamily::Nr,
            classification: Classification::Fallback,
            setup: setup_env_expansion_fixture_npm,
            subject: "dev",
            forwarded_args: &[],
            fallback_reason_fragment: Some("unsupported fast environment expansion"),
            assert_state: assert_env_expansion_fixture,
        },
        NativeRegressionCase {
            name: "pnpm-local-bin-exec",
            upstream_file: "https://github.com/pnpm/pnpm/blob/main/exec/lifecycle/test/index.ts",
            upstream_test: "runLifecycleHook() escapes the args passed to the script",
            manager: Manager::Pnpm,
            family: CommandFamily::Nlx,
            classification: Classification::Equivalence,
            setup: setup_local_bin_fixture_pnpm,
            subject: "hello",
            forwarded_args: &["world", "again"],
            fallback_reason_fragment: None,
            assert_state: assert_local_bin_fixture,
        },
        NativeRegressionCase {
            name: "pnpm-script-arg-newline-escaping",
            upstream_file: "https://github.com/pnpm/pnpm/blob/main/exec/lifecycle/test/index.ts",
            upstream_test: "runLifecycleHook() passes newline correctly",
            manager: Manager::Pnpm,
            family: CommandFamily::Nr,
            classification: Classification::Equivalence,
            setup: setup_pnpm_newline_args_fixture,
            subject: "echo",
            forwarded_args: &["a\nb != 'A\\nB'"],
            fallback_reason_fragment: None,
            assert_state: assert_pnpm_newline_args_fixture,
        },
        NativeRegressionCase {
            name: "yarn-script-precedence-over-bin",
            upstream_file: "https://github.com/yarnpkg/berry/blob/master/packages/acceptance-tests/pkg-tests-specs/sources/commands/run.test.js",
            upstream_test: "it should prefer scripts over binaries",
            manager: Manager::Yarn,
            family: CommandFamily::Nr,
            classification: Classification::Equivalence,
            setup: setup_yarn_script_precedence_fixture,
            subject: "hello",
            forwarded_args: &[],
            fallback_reason_fragment: None,
            assert_state: assert_yarn_script_precedence_fixture,
        },
        NativeRegressionCase {
            name: "yarn-script-args-without-double-dash",
            upstream_file: "https://github.com/yarnpkg/berry/blob/master/packages/acceptance-tests/pkg-tests-specs/sources/commands/run.test.js",
            upstream_test: "it shouldn't require the \"--\" flag to stop interpreting options after \"run\" commands (scripts)",
            manager: Manager::Yarn,
            family: CommandFamily::Nr,
            classification: Classification::Equivalence,
            setup: setup_yarn_option_forwarding_fixture,
            subject: "hello",
            forwarded_args: &["--hello"],
            fallback_reason_fragment: None,
            assert_state: assert_yarn_option_forwarding_fixture,
        },
        NativeRegressionCase {
            name: "bun-script-exit-code",
            upstream_file: "https://github.com/oven-sh/bun/blob/main/test/cli/install/bun-run.test.ts",
            upstream_test: "exit code message works above 128 / exit signal works",
            manager: Manager::Bun,
            family: CommandFamily::Nr,
            classification: Classification::Equivalence,
            setup: setup_bun_exit_code_fixture,
            subject: "dev",
            forwarded_args: &[],
            fallback_reason_fragment: None,
            assert_state: assert_bun_exit_code_fixture,
        },
        NativeRegressionCase {
            name: "bun-prepost-do-not-receive-forwarded-args",
            upstream_file: "https://github.com/oven-sh/bun/blob/main/test/cli/install/bun-run.test.ts",
            upstream_test: "should not passthrough script arguments to pre- or post- scripts",
            manager: Manager::Bun,
            family: CommandFamily::Nr,
            classification: Classification::Equivalence,
            setup: setup_bun_prepost_fixture,
            subject: "myscript",
            forwarded_args: &["-a", "-b", "-c"],
            fallback_reason_fragment: None,
            assert_state: assert_bun_prepost_fixture,
        },
    ]
}

fn run_case(case: NativeRegressionCase) -> CaseRun {
    let work = tempfile::tempdir().unwrap();
    let oracle_root = work.path().join(format!("{}-oracle", case.name));
    let hni_root = work.path().join(format!("{}-hni", case.name));
    fs::create_dir_all(&oracle_root).unwrap();
    fs::create_dir_all(&hni_root).unwrap();
    (case.setup)(&oracle_root);
    (case.setup)(&hni_root);

    let oracle_output = run_command(
        case.manager.command(),
        &oracle_args(case),
        &oracle_root,
        &oracle_env(case.manager),
    );
    let hni_output = run_hni_owned(&hni_args(case, &hni_root), &[("HNI_SKIP_PM_CHECK", "1")]);
    let debug_output = run_hni_owned(
        &hni_debug_args(case, &hni_root),
        &[("HNI_SKIP_PM_CHECK", "1")],
    );

    let explain_output = matches!(case.classification, Classification::Fallback).then(|| {
        run_hni_owned(
            &hni_explain_args(case, &hni_root),
            &[("HNI_SKIP_PM_CHECK", "1")],
        )
    });

    CaseRun {
        _work: work,
        oracle_root,
        hni_root,
        oracle_output,
        hni_output,
        debug_output,
        explain_output,
    }
}

fn oracle_args(case: NativeRegressionCase) -> Vec<String> {
    match (case.manager, case.family) {
        (Manager::Npm, CommandFamily::Nr) => {
            let mut args = vec!["run".to_string(), case.subject.to_string()];
            if !case.forwarded_args.is_empty() {
                args.push("--".to_string());
                args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
            }
            args
        }
        (Manager::Pnpm, CommandFamily::Nr)
        | (Manager::Yarn, CommandFamily::Nr)
        | (Manager::Bun, CommandFamily::Nr) => {
            let mut args = vec!["run".to_string(), case.subject.to_string()];
            args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
            args
        }
        (Manager::Npm, CommandFamily::Nlx) => {
            let mut args = vec![
                "exec".to_string(),
                "--".to_string(),
                case.subject.to_string(),
            ];
            args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
            args
        }
        (Manager::Pnpm, CommandFamily::Nlx) => {
            let mut args = vec!["exec".to_string(), case.subject.to_string()];
            args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
            args
        }
        (Manager::Yarn, CommandFamily::Nlx) => {
            let mut args = vec!["run".to_string(), case.subject.to_string()];
            args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
            args
        }
        (Manager::Bun, CommandFamily::Nlx) => {
            let mut args = vec!["x".to_string(), case.subject.to_string()];
            args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
            args
        }
    }
}

fn hni_args(case: NativeRegressionCase, cwd: &Path) -> Vec<String> {
    let mut args = vec![
        match case.family {
            CommandFamily::Nr => "nr".to_string(),
            CommandFamily::Nlx => "nlx".to_string(),
        },
        "-C".to_string(),
        cwd.to_string_lossy().to_string(),
        "--fast".to_string(),
        case.subject.to_string(),
    ];
    args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
    args
}

fn hni_explain_args(case: NativeRegressionCase, cwd: &Path) -> Vec<String> {
    let mut args = vec![
        match case.family {
            CommandFamily::Nr => "nr".to_string(),
            CommandFamily::Nlx => "nlx".to_string(),
        },
        "-C".to_string(),
        cwd.to_string_lossy().to_string(),
        "--fast".to_string(),
        "--explain".to_string(),
        case.subject.to_string(),
    ];
    args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
    args
}

fn hni_debug_args(case: NativeRegressionCase, cwd: &Path) -> Vec<String> {
    let mut args = vec![
        match case.family {
            CommandFamily::Nr => "nr".to_string(),
            CommandFamily::Nlx => "nlx".to_string(),
        },
        "-C".to_string(),
        cwd.to_string_lossy().to_string(),
        "--fast".to_string(),
        "--debug-resolved".to_string(),
        case.subject.to_string(),
    ];
    args.extend(case.forwarded_args.iter().map(|arg| arg.to_string()));
    args
}

fn oracle_env(manager: Manager) -> Vec<(&'static str, &'static str)> {
    match manager {
        Manager::Bun => vec![("BUN_INSTALL_CACHE_DIR", "/tmp/hni-bun-cache")],
        _ => Vec::new(),
    }
}

fn exit_codes_match(oracle: &std::process::Output, hni: &std::process::Output) -> bool {
    oracle.status.code() == hni.status.code()
}

fn setup_hooked_script_fixture_npm(root: &Path) {
    init_project(root, Manager::Npm, "hooked");
    fs::write(
        root.join("write-args.cjs"),
        "const fs = require('fs'); fs.writeFileSync('args.txt', JSON.stringify(process.argv.slice(2))); fs.appendFileSync('order.txt', 'dev');\n",
    )
    .unwrap();
    fs::write(
        root.join("package.json"),
        r#"{"name":"hooked","packageManager":"npm@11.6.2","scripts":{"predev":"printf 'pre' >> order.txt","dev":"node write-args.cjs","postdev":"printf 'post' >> order.txt"}}"#,
    )
    .unwrap();
}

fn setup_env_expansion_fixture_npm(root: &Path) {
    init_project(root, Manager::Npm, "envy");
    fs::write(
        root.join("package.json"),
        r#"{"name":"envy","packageManager":"npm@11.6.2","scripts":{"dev":"printf '%s' \"$npm_package_name\" > env.txt"}}"#,
    )
    .unwrap();
}

fn setup_local_bin_fixture_pnpm(root: &Path) {
    init_project(root, Manager::Pnpm, "pnpm-local-bin");
    fs::write(
        root.join("package.json"),
        format!(
            r#"{{"name":"pnpm-local-bin","packageManager":"{}"}}"#,
            Manager::Pnpm.package_manager()
        ),
    )
    .unwrap();

    let bin_dir = root.join("node_modules").join(".bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let bin = bin_dir.join("hello");
    fs::write(
        &bin,
        "#!/bin/sh\nprintf '%s' \"$*\" > bin-args.txt\nprintf 'bin' > bin-source.txt\n",
    )
    .unwrap();
    make_executable(&bin);
}

fn setup_yarn_script_precedence_fixture(root: &Path) {
    init_project(root, Manager::Yarn, "yarn-script-precedence");
    fs::write(
        root.join("package.json"),
        format!(
            r#"{{"name":"yarn-script-precedence","packageManager":"{}","scripts":{{"hello":"printf 'script' > source.txt"}}}}"#,
            Manager::Yarn.package_manager()
        ),
    )
    .unwrap();

    let bin_dir = root.join("node_modules").join(".bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let bin = bin_dir.join("hello");
    fs::write(&bin, "#!/bin/sh\nprintf 'bin' > source.txt\n").unwrap();
    make_executable(&bin);
}

fn setup_yarn_option_forwarding_fixture(root: &Path) {
    init_project(root, Manager::Yarn, "yarn-option-forwarding");
    fs::write(
        root.join("write-args.cjs"),
        "require('fs').writeFileSync('args.txt', JSON.stringify(process.argv.slice(2)))\n",
    )
    .unwrap();
    fs::write(
        root.join("package.json"),
        format!(
            r#"{{"name":"yarn-option-forwarding","packageManager":"{}","scripts":{{"hello":"node write-args.cjs"}}}}"#,
            Manager::Yarn.package_manager()
        ),
    )
    .unwrap();
}

fn setup_bun_exit_code_fixture(root: &Path) {
    init_project(root, Manager::Bun, "bun-exit");
    fs::write(
        root.join("package.json"),
        format!(
            r#"{{"name":"bun-exit","packageManager":"{}","scripts":{{"dev":"sh -c 'exit 42'"}}}}"#,
            Manager::Bun.package_manager()
        ),
    )
    .unwrap();
}

fn setup_bun_prepost_fixture(root: &Path) {
    init_project(root, Manager::Bun, "bun-prepost");
    fs::write(
        root.join("record-args.cjs"),
        "const fs = require('fs'); const key = process.argv[2]; const args = process.argv.slice(3); fs.writeFileSync(`${key}.json`, JSON.stringify(args));\n",
    )
    .unwrap();
    fs::write(
        root.join("package.json"),
        format!(
            r#"{{"name":"bun-prepost","packageManager":"{}","scripts":{{"premyscript":"node record-args.cjs pre","myscript":"node record-args.cjs main","postmyscript":"node record-args.cjs post"}}}}"#,
            Manager::Bun.package_manager()
        ),
    )
    .unwrap();
}

fn setup_pnpm_newline_args_fixture(root: &Path) {
    init_project(root, Manager::Pnpm, "pnpm-newline-args");
    fs::write(
        root.join("write-args.cjs"),
        "require('fs').writeFileSync('args.txt', JSON.stringify(process.argv.slice(2)))\n",
    )
    .unwrap();
    fs::write(
        root.join("package.json"),
        format!(
            r#"{{"name":"pnpm-newline-args","packageManager":"{}","scripts":{{"echo":"node write-args.cjs"}}}}"#,
            Manager::Pnpm.package_manager()
        ),
    )
    .unwrap();
}

fn init_project(root: &Path, manager: Manager, name: &str) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join(manager.lockfile()), "lock\n").unwrap();
    fs::write(
        root.join("package.json"),
        format!(
            r#"{{"name":"{name}","packageManager":"{}","scripts":{{}}}}"#,
            manager.package_manager()
        ),
    )
    .unwrap();
}

fn assert_hooked_script_fixture(root: &Path) {
    assert_eq!(
        fs::read_to_string(root.join("order.txt")).unwrap(),
        "predevpost"
    );
    assert_eq!(
        fs::read_to_string(root.join("args.txt")).unwrap(),
        "[\"alpha\",\"beta\"]"
    );
}

fn assert_env_expansion_fixture(root: &Path) {
    assert_eq!(fs::read_to_string(root.join("env.txt")).unwrap(), "envy");
}

fn assert_local_bin_fixture(root: &Path) {
    assert_eq!(
        fs::read_to_string(root.join("bin-args.txt")).unwrap(),
        "world again"
    );
    assert_eq!(
        fs::read_to_string(root.join("bin-source.txt")).unwrap(),
        "bin"
    );
}

fn assert_yarn_script_precedence_fixture(root: &Path) {
    assert_eq!(
        fs::read_to_string(root.join("source.txt")).unwrap(),
        "script"
    );
}

fn assert_yarn_option_forwarding_fixture(root: &Path) {
    assert_eq!(
        fs::read_to_string(root.join("args.txt")).unwrap(),
        "[\"--hello\"]"
    );
}

fn assert_bun_exit_code_fixture(_root: &Path) {}

fn assert_bun_prepost_fixture(root: &Path) {
    assert_eq!(fs::read_to_string(root.join("pre.json")).unwrap(), "[]");
    assert_eq!(
        fs::read_to_string(root.join("main.json")).unwrap(),
        "[\"-a\",\"-b\",\"-c\"]"
    );
    assert_eq!(fs::read_to_string(root.join("post.json")).unwrap(), "[]");
}

fn assert_pnpm_newline_args_fixture(root: &Path) {
    assert_eq!(
        fs::read_to_string(root.join("args.txt")).unwrap(),
        "[\"a\\nb != 'A\\\\nB'\"]"
    );
}

fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}
