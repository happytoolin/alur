use std::{fs, path::Path};

mod support;

use support::run_hni;

struct ModeCase {
    category: &'static str,
    name: &'static str,
    pm_nr: &'static str,
    fast_nr: &'static str,
    pm_nlx: &'static str,
    fast_nlx: &'static str,
    local_bin: bool,
}

#[test]
fn fixture_projects_cover_pm_and_fast_resolution_modes() {
    support::with_env_lock(|| {
        let cases = [
            ModeCase {
                category: "packager",
                name: "npm",
                pm_nr: "npm run dev",
                fast_nr: "hni fast:run-script dev",
                pm_nlx: "npx hello --flag",
                fast_nlx: "hni fast:run-local-bin hello --flag",
                local_bin: true,
            },
            ModeCase {
                category: "dev-engines",
                name: "pnpm-version-range",
                pm_nr: "pnpm run dev",
                fast_nr: "hni fast:run-script dev",
                pm_nlx: "pnpm dlx hello --flag",
                fast_nlx: "hni fast:run-local-bin hello --flag",
                local_bin: true,
            },
            ModeCase {
                category: "lockfile",
                name: "yarn@berry",
                pm_nr: "yarn run dev",
                fast_nr: "hni fast:run-script dev",
                pm_nlx: "npx hello --flag",
                fast_nlx: "hni fast:run-local-bin hello --flag",
                local_bin: true,
            },
            ModeCase {
                category: "install-metadata",
                name: "yarn@berry",
                pm_nr: "yarn run dev",
                fast_nr: "hni fast:run-script dev",
                pm_nlx: "yarn dlx hello --flag",
                fast_nlx: "hni fast:run-local-bin hello --flag",
                local_bin: true,
            },
            ModeCase {
                category: "install-metadata",
                name: "yarn@berry_pnp-v3",
                pm_nr: "yarn run dev",
                fast_nr: "yarn run dev",
                pm_nlx: "yarn dlx hello --flag",
                fast_nlx: "yarn dlx hello --flag",
                local_bin: false,
            },
            ModeCase {
                category: "packager",
                name: "deno",
                pm_nr: "deno task dev",
                fast_nr: "hni fast:run-deno-task dev",
                pm_nlx: "deno run npm:hello --flag",
                fast_nlx: "deno run npm:hello --flag",
                local_bin: false,
            },
        ];

        for case in cases {
            let dir = tempfile::tempdir().unwrap();
            support::copy_fixture_into(case.category, case.name, dir.path());
            prepare_project(dir.path(), case.local_bin);

            assert_debug_resolved(
                dir.path(),
                &[
                    "nr",
                    "-C",
                    dir.path().to_str().unwrap(),
                    "--debug-resolved",
                    "dev",
                ],
                &[("HNI_SKIP_PM_CHECK", "1"), ("HNI_FAST", "false")],
                case.pm_nr,
            );
            assert_debug_resolved(
                dir.path(),
                &[
                    "nr",
                    "-C",
                    dir.path().to_str().unwrap(),
                    "--debug-resolved",
                    "dev",
                ],
                &[("HNI_SKIP_PM_CHECK", "1"), ("HNI_FAST", "true")],
                case.fast_nr,
            );
            assert_debug_resolved(
                dir.path(),
                &[
                    "nlx",
                    "-C",
                    dir.path().to_str().unwrap(),
                    "--debug-resolved",
                    "hello",
                    "--flag",
                ],
                &[("HNI_SKIP_PM_CHECK", "1"), ("HNI_FAST", "false")],
                case.pm_nlx,
            );
            assert_debug_resolved(
                dir.path(),
                &[
                    "nlx",
                    "-C",
                    dir.path().to_str().unwrap(),
                    "--debug-resolved",
                    "hello",
                    "--flag",
                ],
                &[("HNI_SKIP_PM_CHECK", "1"), ("HNI_FAST", "true")],
                case.fast_nlx,
            );
        }
    });
}

fn assert_debug_resolved(cwd: &Path, args: &[&str], env: &[(&str, &str)], expected: &str) {
    let output = run_hni(args.to_vec(), env);
    assert!(
        output.status.success(),
        "command failed in {}: {}",
        cwd.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
}

fn prepare_project(project: &Path, local_bin: bool) {
    if local_bin {
        let bin_dir = project.join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("hello");
        fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
        make_executable(&bin);
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
