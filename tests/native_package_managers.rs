use std::fs;

mod support;

use support::run_hni;

struct PmCase {
    label: &'static str,
    package_manager: &'static str,
    lockfile: &'static str,
    local_bins: bool,
    yarn_pnp: bool,
    expected_nr: &'static str,
    expected_nlx: &'static str,
}

#[test]
fn fast_mode_matrix_covers_supported_and_fallback_package_managers() {
    if cfg!(windows) {
        return;
    }

    support::with_env_lock(|| {
        let cases = [
            PmCase {
                label: "npm",
                package_manager: "npm@10.0.0",
                lockfile: "package-lock.json",
                local_bins: true,
                yarn_pnp: false,
                expected_nr: "hni fast:run-script dev",
                expected_nlx: "hni fast:run-local-bin hello --flag",
            },
            PmCase {
                label: "pnpm",
                package_manager: "pnpm@9.0.0",
                lockfile: "pnpm-lock.yaml",
                local_bins: true,
                yarn_pnp: false,
                expected_nr: "hni fast:run-script dev",
                expected_nlx: "hni fast:run-local-bin hello --flag",
            },
            PmCase {
                label: "yarn-classic",
                package_manager: "yarn@1.22.0",
                lockfile: "yarn.lock",
                local_bins: true,
                yarn_pnp: false,
                expected_nr: "hni fast:run-script dev",
                expected_nlx: "hni fast:run-local-bin hello --flag",
            },
            PmCase {
                label: "yarn-berry-node-modules",
                package_manager: "yarn@4.0.0",
                lockfile: "yarn.lock",
                local_bins: true,
                yarn_pnp: false,
                expected_nr: "hni fast:run-script dev",
                expected_nlx: "hni fast:run-local-bin hello --flag",
            },
            PmCase {
                label: "yarn-berry-pnp",
                package_manager: "yarn@4.0.0",
                lockfile: "yarn.lock",
                local_bins: false,
                yarn_pnp: true,
                expected_nr: "yarn run dev",
                expected_nlx: "yarn dlx hello --flag",
            },
            PmCase {
                label: "bun",
                package_manager: "bun@1.1.0",
                lockfile: "bun.lockb",
                local_bins: true,
                yarn_pnp: false,
                expected_nr: "hni fast:run-script dev",
                expected_nlx: "hni fast:run-local-bin hello --flag",
            },
            PmCase {
                label: "deno",
                package_manager: "deno@1.46.0",
                lockfile: "deno.lock",
                local_bins: false,
                yarn_pnp: false,
                expected_nr: "hni fast:run-deno-task dev",
                expected_nlx: "deno run npm:hello --flag",
            },
        ];

        for case in cases {
            let work = tempfile::tempdir().unwrap();
            let project = work.path().join(case.label);
            fs::create_dir_all(&project).unwrap();
            fs::write(project.join(case.lockfile), "lock\n").unwrap();
            fs::write(
                project.join("package.json"),
                format!(
                    r#"{{"name":"{}","packageManager":"{}","scripts":{{"dev":"echo ok"}}}}"#,
                    case.label, case.package_manager
                ),
            )
            .unwrap();

            if case.yarn_pnp {
                fs::write(project.join(".pnp.cjs"), "module.exports = {};\n").unwrap();
            }

            if case.local_bins {
                let bin_dir = project.join("node_modules").join(".bin");
                fs::create_dir_all(&bin_dir).unwrap();
                let bin = bin_dir.join("hello");
                fs::write(&bin, "#!/bin/sh\n").unwrap();
                make_executable(&bin);
            }

            let nr = run_hni(
                vec![
                    "nr",
                    "-C",
                    project.to_str().unwrap(),
                    "--fast",
                    "--debug-resolved",
                    "dev",
                ],
                &[("HNI_SKIP_PM_CHECK", "1")],
            );
            assert!(
                nr.status.success(),
                "nr failed for {}: {}",
                case.label,
                String::from_utf8_lossy(&nr.stderr)
            );
            assert_eq!(String::from_utf8_lossy(&nr.stdout).trim(), case.expected_nr);

            let nlx = run_hni(
                vec![
                    "nlx",
                    "-C",
                    project.to_str().unwrap(),
                    "--fast",
                    "--debug-resolved",
                    "hello",
                    "--flag",
                ],
                &[("HNI_SKIP_PM_CHECK", "1")],
            );
            assert!(
                nlx.status.success(),
                "nlx failed for {}: {}",
                case.label,
                String::from_utf8_lossy(&nlx.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&nlx.stdout).trim(),
                case.expected_nlx
            );
        }
    });
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}
