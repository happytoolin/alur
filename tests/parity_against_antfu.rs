use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

mod support;

#[test]
fn compare_pm_mode_with_installed_antfu_when_available() {
    if std::env::var("ALUR_ENABLE_PARITY_REFERENCE")
        .ok()
        .as_deref()
        != Some("1")
    {
        return;
    }

    support::with_env_lock(|| {
        let required_fixtures = required_fixtures();

        let Some(cmds) = antfu_bins() else {
            if !required_fixtures.is_empty() {
                panic!(
                    "required parity fixtures {:?}, but antfu ni binaries are not installed",
                    required_fixtures
                );
            }
            return;
        };

        let our_bin = alur_executable_path();
        if !our_bin.exists() {
            if !required_fixtures.is_empty() {
                panic!(
                    "required parity fixtures {:?}, but local alur binary was not found at {}",
                    required_fixtures,
                    our_bin.display()
                );
            }
            return;
        }
        let work = tempfile::tempdir().unwrap();
        let our_alias_dir = work.path().join("our-bin");
        fs::create_dir_all(&our_alias_dir).unwrap();

        create_alias(&our_bin, &our_alias_dir, "ni");
        create_alias(&our_bin, &our_alias_dir, "nr");
        create_alias(&our_bin, &our_alias_dir, "nlx");
        create_alias(&our_bin, &our_alias_dir, "nci");

        let mut executed_fixtures = BTreeSet::new();

        for fixture in build_fixtures(work.path(), &cmds) {
            if !fixture
                .prereq_bins
                .iter()
                .all(|bin| which::which(bin).is_ok())
            {
                continue;
            }

            executed_fixtures.insert(fixture.name.clone());

            for case in &fixture.cases {
                let antfu_out = run(
                    &case.antfu_bin,
                    &fixture.path,
                    &case.args,
                    "?",
                    &[("ALUR_SKIP_PM_CHECK", "1")],
                );

                let our_bin_path = if cfg!(windows) {
                    our_alias_dir.join(format!("{}.exe", case.our_bin))
                } else {
                    our_alias_dir.join(&case.our_bin)
                };

                let our_out = run(
                    &our_bin_path,
                    &fixture.path,
                    &case.args,
                    "--print-command",
                    &[("ALUR_SKIP_PM_CHECK", "1"), ("ALUR_FAST_MODE", "false")],
                );

                assert_eq!(
                    normalize(&our_out),
                    normalize(&antfu_out),
                    "mismatch for fixture={} our={} args={:?}\nantfu={}\nours={}",
                    fixture.name,
                    case.our_bin,
                    case.args,
                    antfu_out.trim(),
                    our_out.trim()
                );
            }
        }

        for required in required_fixtures {
            assert!(
                executed_fixtures.contains(&required),
                "required parity fixture '{}' did not run. executed fixtures: {:?}",
                required,
                executed_fixtures
            );
        }
    });
}

#[derive(Clone)]
struct Fixture {
    name: String,
    path: PathBuf,
    prereq_bins: Vec<String>,
    cases: Vec<Case>,
}

#[derive(Clone)]
struct Case {
    antfu_bin: PathBuf,
    our_bin: String,
    args: Vec<String>,
}

fn build_fixtures(root: &Path, cmds: &AntfuBins) -> Vec<Fixture> {
    let mut fixtures = Vec::new();

    let base_cases = vec![
        Case {
            antfu_bin: cmds.ni.clone(),
            our_bin: "ni".into(),
            args: vec![],
        },
        Case {
            antfu_bin: cmds.ni.clone(),
            our_bin: "ni".into(),
            args: vec!["vite".into()],
        },
        Case {
            antfu_bin: cmds.ni.clone(),
            our_bin: "ni".into(),
            args: vec!["@types/node".into(), "-D".into()],
        },
        Case {
            antfu_bin: cmds.ni.clone(),
            our_bin: "ni".into(),
            args: vec!["-P".into()],
        },
        Case {
            antfu_bin: cmds.ni.clone(),
            our_bin: "ni".into(),
            args: vec!["-g".into(), "eslint".into()],
        },
        Case {
            antfu_bin: cmds.ni.clone(),
            our_bin: "ni".into(),
            args: vec!["--frozen".into()],
        },
        Case {
            antfu_bin: cmds.ni.clone(),
            our_bin: "ni".into(),
            args: vec!["--frozen-if-present".into()],
        },
        Case {
            antfu_bin: cmds.nci.clone(),
            our_bin: "nci".into(),
            args: vec![],
        },
        Case {
            antfu_bin: cmds.nr.clone(),
            our_bin: "nr".into(),
            args: vec!["dev".into(), "--port=3000".into()],
        },
        Case {
            antfu_bin: cmds.nr.clone(),
            our_bin: "nr".into(),
            args: vec!["--if-present".into(), "missing-script".into()],
        },
        Case {
            antfu_bin: cmds.nlx.clone(),
            our_bin: "nlx".into(),
            args: vec!["vitest".into()],
        },
        Case {
            antfu_bin: cmds.nlx.clone(),
            our_bin: "nlx".into(),
            args: vec!["vitest".into(), "--help".into()],
        },
    ];

    let npm = root.join("npm");
    fs::create_dir_all(&npm).unwrap();
    fs::write(
        npm.join("package.json"),
        r#"{"name":"x","scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    fs::write(npm.join("package-lock.json"), "lock").unwrap();
    fixtures.push(Fixture {
        name: "npm".into(),
        path: npm,
        prereq_bins: vec!["npm".into(), "npx".into()],
        cases: base_cases.clone(),
    });

    let yarn = root.join("yarn");
    fs::create_dir_all(&yarn).unwrap();
    fs::write(
        yarn.join("package.json"),
        r#"{"name":"x","scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    fs::write(yarn.join("yarn.lock"), "lock").unwrap();
    fixtures.push(Fixture {
        name: "yarn".into(),
        path: yarn,
        prereq_bins: vec!["yarn".into()],
        cases: base_cases.clone(),
    });

    let pnpm = root.join("pnpm");
    fs::create_dir_all(&pnpm).unwrap();
    fs::write(
        pnpm.join("package.json"),
        r#"{"name":"x","scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    fs::write(pnpm.join("pnpm-lock.yaml"), "lock").unwrap();
    fixtures.push(Fixture {
        name: "pnpm".into(),
        path: pnpm,
        prereq_bins: vec!["pnpm".into()],
        cases: base_cases.clone(),
    });

    let workspace_root = root.join("pnpm-workspace");
    let workspace_pkg = workspace_root.join("packages").join("app");
    fs::create_dir_all(&workspace_pkg).unwrap();
    fs::write(
        workspace_root.join("package.json"),
        r#"{"name":"workspace","packageManager":"pnpm@9.0.0","workspaces":["packages/*"]}"#,
    )
    .unwrap();
    fs::write(workspace_root.join("pnpm-lock.yaml"), "lock").unwrap();
    fs::write(
        workspace_pkg.join("package.json"),
        r#"{"name":"app","scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    fixtures.push(Fixture {
        name: "pnpm-workspace-subpkg".into(),
        path: workspace_pkg,
        prereq_bins: vec!["pnpm".into()],
        cases: base_cases.clone(),
    });

    let bun = root.join("bun");
    fs::create_dir_all(&bun).unwrap();
    fs::write(
        bun.join("package.json"),
        r#"{"name":"x","scripts":{"dev":"vite"}}"#,
    )
    .unwrap();
    fs::write(bun.join("bun.lockb"), "lock").unwrap();
    fixtures.push(Fixture {
        name: "bun".into(),
        path: bun,
        prereq_bins: vec!["bun".into()],
        cases: base_cases.clone(),
    });

    let deno = root.join("deno");
    fs::create_dir_all(&deno).unwrap();
    fs::write(
        deno.join("package.json"),
        r#"{"name":"x","scripts":{"dev":"deno task dev"}}"#,
    )
    .unwrap();
    fs::write(deno.join("deno.lock"), "lock").unwrap();
    fixtures.push(Fixture {
        name: "deno".into(),
        path: deno,
        prereq_bins: vec!["deno".into()],
        cases: base_cases,
    });

    fixtures
}

#[derive(Clone)]
struct AntfuBins {
    ni: PathBuf,
    nr: PathBuf,
    nlx: PathBuf,
    nci: PathBuf,
}

fn antfu_bins() -> Option<AntfuBins> {
    Some(AntfuBins {
        ni: which::which("ni").ok()?,
        nr: which::which("nr").ok()?,
        nlx: which::which("nlx").ok()?,
        nci: which::which("nci").ok()?,
    })
}

fn run(
    bin: &Path,
    fixture: &Path,
    args: &[String],
    print_arg: &str,
    envs: &[(&str, &str)],
) -> String {
    let mut cmd = Command::new(bin);
    cmd.arg("-C").arg(fixture);
    cmd.args(args);
    cmd.arg(print_arg);

    for (k, v) in envs {
        cmd.env(k, v);
    }

    let output = cmd.output().expect("failed to run command");
    assert!(
        output.status.success(),
        "command failed: {}\n{}",
        bin.display(),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn normalize(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn create_alias(target: &Path, dir: &Path, alias: &str) {
    let alias_path = if cfg!(windows) {
        dir.join(format!("{alias}.exe"))
    } else {
        dir.join(alias)
    };

    if alias_path.exists() {
        fs::remove_file(&alias_path).unwrap();
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, alias_path).unwrap();
    }

    #[cfg(windows)]
    {
        fs::copy(target, alias_path).unwrap();
    }
}

fn alur_executable_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_alur") {
        return PathBuf::from(path);
    }

    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push(if cfg!(windows) { "alur.exe" } else { "alur" });
    path
}

fn required_fixtures() -> BTreeSet<String> {
    std::env::var("ALUR_PARITY_REQUIRE_FIXTURES")
        .ok()
        .into_iter()
        .flat_map(|raw| {
            raw.split(',')
                .map(str::trim)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|item| !item.is_empty())
        .collect()
}
