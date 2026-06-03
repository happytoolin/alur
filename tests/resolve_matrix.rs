use std::{fs, path::Path};

use hni::core::{
    config::HniConfig,
    resolve::{self, ResolveContext},
    types::{ExecutionStrategy, NativeExecution, PackageManager},
};

mod support;

fn with_skip_pm_check<T>(f: impl FnOnce() -> T) -> T {
    support::with_env_lock(|| {
        struct SkipPmCheckGuard;
        impl Drop for SkipPmCheckGuard {
            fn drop(&mut self) {
                support::remove_var("HNI_SKIP_PM_CHECK");
            }
        }

        support::set_var("HNI_SKIP_PM_CHECK", "1");
        let guard = SkipPmCheckGuard;
        let out = f();
        drop(guard);
        out
    })
}

fn write_package_json(dir: &Path, raw: &str) {
    fs::write(dir.join("package.json"), raw).unwrap();
}

fn with_path_override<T>(path: &str, f: impl FnOnce() -> T) -> T {
    struct PathGuard(Option<std::ffi::OsString>);
    impl Drop for PathGuard {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => support::set_var("PATH", value),
                None => support::remove_var("PATH"),
            }
        }
    }

    let original = std::env::var_os("PATH");
    support::set_var("PATH", path);
    let guard = PathGuard(original);
    let out = f();
    drop(guard);
    out
}

#[test]
fn ni_maps_npm_add_to_install() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["vite".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["i", "vite"]);
    });
}

#[test]
fn ni_empty_args_installs_for_npm() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(Vec::new(), &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["i"]);
    });
}

#[test]
fn ni_only_flags_stays_install_mode_not_add_mode() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"yarn@1.22.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["--check-files".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["install", "--check-files"]);
    });
}

#[test]
fn ni_maps_yarn_add() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"yarn@1.22.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["vite".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["add", "vite"]);
    });
}

#[test]
fn ni_bun_dev_flag_is_translated() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"bun@1.1.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["@types/node".into(), "-D".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "bun");
        assert_eq!(resolved.args, vec!["add", "@types/node", "-d"]);
    });
}

#[test]
fn ni_npm_prod_flag_is_translated_to_omit_dev() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["-P".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["i", "--omit=dev"]);
    });
}

#[test]
fn ni_non_npm_prod_flag_is_translated_to_production() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"yarn@1.22.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["-P".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["install", "--production"]);
    });
}

#[test]
fn ni_global_install_uses_configured_global_package_manager() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"pnpm@9.0.0"}"#);

        let cfg = HniConfig {
            global_package_manager: PackageManager::Yarn,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_ni(vec!["-g".into(), "eslint".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["global", "add", "eslint"]);
    });
}

#[test]
fn nun_maps_npm_uninstall() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nun(vec!["lodash".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["uninstall", "lodash"]);
    });
}

#[test]
fn nun_global_uninstall_uses_configured_global_package_manager() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"pnpm@9.0.0"}"#);

        let cfg = HniConfig {
            global_package_manager: PackageManager::Yarn,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nun(vec!["-g".into(), "typescript".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["global", "remove", "typescript"]);
    });
}

#[test]
fn nun_requires_dependency_target() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let err = resolve::resolve_nun(Vec::new(), &ctx).unwrap_err();

        assert!(err.to_string().contains("nun requires a dependency"));
    });
}

#[test]
fn ni_frozen_if_present_uses_clean_install_with_lock() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);
        fs::write(dir.path().join("package-lock.json"), "lock").unwrap();

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["--frozen-if-present".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["ci"]);
    });
}

#[test]
fn ni_frozen_if_present_falls_back_to_install_without_lock() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["--frozen-if-present".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["i"]);
    });
}

#[test]
fn ni_frozen_always_uses_clean_install() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["--frozen".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["ci"]);
    });
}

#[test]
fn nr_maps_pnpm_run() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"pnpm@9.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "pnpm");
        assert_eq!(resolved.args, vec!["run", "dev"]);
    });
}

#[test]
fn nr_without_args_defaults_to_start() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nr(Vec::new(), &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["run", "start"]);
    });
}

#[test]
fn nr_npm_with_extra_args_inserts_double_dash() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nr(vec!["dev".into(), "--port=3000".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["run", "dev", "--", "--port=3000"]);
    });
}

#[test]
fn nr_if_present_for_npm_is_inserted_after_run() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved =
            resolve::resolve_nr(vec!["build".into(), "--if-present".into()], &ctx).unwrap();

        match &resolved.strategy {
            ExecutionStrategy::Native(NativeExecution::RunScript(exec)) => {
                assert!(exec.steps.is_empty());
                assert!(exec.forwarded_args.is_empty());
            }
            other => panic!("expected native script noop, got {other:?}"),
        }
    });
}

#[test]
fn nr_if_present_for_npm_with_extra_args_keeps_double_dash() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nr(
            vec!["dev".into(), "--if-present".into(), "--port=3000".into()],
            &ctx,
        )
        .unwrap();

        match &resolved.strategy {
            ExecutionStrategy::Native(NativeExecution::RunScript(exec)) => {
                assert!(exec.steps.is_empty());
                assert_eq!(exec.forwarded_args, vec!["--port=3000"]);
            }
            other => panic!("expected native script noop, got {other:?}"),
        }
    });
}

#[test]
fn nr_if_present_for_non_run_command_is_prefixed() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"deno@1.46.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved =
            resolve::resolve_nr(vec!["build".into(), "--if-present".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "deno");
        assert_eq!(resolved.args, vec!["task", "--if-present", "build"]);
    });
}

#[test]
fn nr_fast_mode_does_not_require_detected_package_manager() {
    with_skip_pm_check(|| {
        with_path_override("", || {
            let dir = tempfile::tempdir().unwrap();
            write_package_json(dir.path(), r#"{"name":"x","scripts":{"dev":"vite"}}"#);

            let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
            let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

            assert!(matches!(
                resolved.strategy,
                ExecutionStrategy::Native(NativeExecution::RunScript(_))
            ));
        });
    });
}

#[test]
fn nlx_fast_mode_does_not_require_detected_package_manager() {
    with_skip_pm_check(|| {
        with_path_override("", || {
            let dir = tempfile::tempdir().unwrap();
            let bin_dir = dir.path().join("node_modules").join(".bin");
            fs::create_dir_all(&bin_dir).unwrap();
            write_package_json(dir.path(), r#"{"name":"x"}"#);
            let bin = bin_dir.join("hello");
            fs::write(&bin, "#!/bin/sh\nexit 0\n").unwrap();
            make_executable(&bin);

            let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
            let resolved = resolve::resolve_nlx(vec!["hello".into()], &ctx).unwrap();

            assert!(matches!(
                resolved.strategy,
                ExecutionStrategy::Native(NativeExecution::RunLocalBin(_))
            ));
        });
    });
}

#[cfg(unix)]
#[test]
fn node_run_uses_native_fast_path() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"npm@10.0.0","scripts":{"dev":"vite"}}"#,
        );

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

        assert!(matches!(
            resolved.strategy,
            ExecutionStrategy::Native(NativeExecution::RunScript(_))
        ));
        assert_eq!(resolved.execution_mode_name(), "fast");
    });
}

#[cfg(unix)]
#[test]
fn node_run_fast_path_does_not_require_detected_package_manager() {
    with_skip_pm_check(|| {
        with_path_override("", || {
            let dir = tempfile::tempdir().unwrap();
            write_package_json(dir.path(), r#"{"name":"x","scripts":{"dev":"vite"}}"#);

            let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
            let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

            assert!(matches!(
                resolved.strategy,
                ExecutionStrategy::Native(NativeExecution::RunScript(_))
            ));
            assert_eq!(resolved.execution_mode_name(), "fast");
        });
    });
}

#[cfg(unix)]
#[test]
fn node_run_uses_native_fast_path_with_hooks() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"npm@10.0.0","scripts":{"predev":"echo pre","dev":"vite"}}"#,
        );

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

        assert!(matches!(
            resolved.strategy,
            ExecutionStrategy::Native(NativeExecution::RunScript(_))
        ));
    });
}

#[cfg(unix)]
#[test]
fn node_run_falls_back_to_package_manager_when_fast_env_is_unsupported() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"npm@10.0.0","scripts":{"dev":"echo $npm_package_name"}}"#,
        );

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["run", "dev"]);
        assert_eq!(resolved.execution_mode_name(), "package-manager");
        assert_eq!(
            resolved.fast_fallback_reason.as_deref(),
            Some("script 'dev' uses unsupported fast environment expansion (npm_package_)")
        );
    });
}

#[test]
fn nr_fast_mode_uses_native_script_execution() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"npm@10.0.0","scripts":{"dev":"vite"}}"#,
        );

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nr(vec!["dev".into(), "--port=3000".into()], &ctx).unwrap();

        assert!(matches!(
            resolved.strategy,
            ExecutionStrategy::Native(NativeExecution::RunScript(_))
        ));
        assert_eq!(resolved.program, "dev");
        assert_eq!(resolved.args, vec!["--port=3000"]);
    });
}

#[test]
fn nr_fast_mode_is_available_on_windows_too() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"npm@10.0.0","scripts":{"dev":"echo ok"}}"#,
        );

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

        assert!(matches!(
            resolved.strategy,
            ExecutionStrategy::Native(NativeExecution::RunScript(_))
        ));
    });
}

#[test]
fn nr_fast_mode_if_present_missing_script_is_native_noop() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"npm@10.0.0","scripts":{}}"#,
        );

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nr(
            vec!["missing".into(), "--if-present".into(), "--watch".into()],
            &ctx,
        )
        .unwrap();

        match &resolved.strategy {
            ExecutionStrategy::Native(NativeExecution::RunScript(exec)) => {
                assert!(exec.steps.is_empty());
                assert_eq!(exec.forwarded_args, vec!["--watch"]);
            }
            other => panic!("expected native script execution, got {other:?}"),
        }
    });
}

#[test]
fn nr_fast_mode_if_present_missing_script_skips_prehook_too() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"npm@10.0.0","scripts":{"premissing":"echo pre"}}"#,
        );

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved =
            resolve::resolve_nr(vec!["missing".into(), "--if-present".into()], &ctx).unwrap();

        match &resolved.strategy {
            ExecutionStrategy::Native(NativeExecution::RunScript(exec)) => {
                assert!(exec.steps.is_empty());
            }
            other => panic!("expected native script execution, got {other:?}"),
        }
    });
}

#[test]
fn nr_fast_mode_falls_back_for_yarn_berry_pnp() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(
            dir.path(),
            r#"{"packageManager":"yarn@4.0.0","scripts":{"dev":"vite"}}"#,
        );
        fs::write(dir.path().join(".pnp.cjs"), "module.exports = {};\n").unwrap();

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

        assert!(matches!(resolved.strategy, ExecutionStrategy::External));
        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["run", "dev"]);
        assert_eq!(
            resolved.fast_fallback_reason.as_deref(),
            Some(
                "yarn berry Plug'n'Play does not expose node_modules/.bin; falling back to yarn execution"
            )
        );
    });
}

#[test]
fn nr_fast_mode_falls_back_for_yarn_berry_pnp_workspace() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("workspace");
        let app = root.join("packages").join("app");
        fs::create_dir_all(&app).unwrap();
        fs::write(root.join("yarn.lock"), "lock").unwrap();
        fs::write(root.join(".pnp.cjs"), "module.exports = {};\n").unwrap();
        write_package_json(root.as_path(), r#"{"packageManager":"yarn@4.0.0"}"#);
        write_package_json(app.as_path(), r#"{"name":"app","scripts":{"dev":"vite"}}"#);

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(app, cfg);
        let resolved = resolve::resolve_nr(vec!["dev".into()], &ctx).unwrap();

        assert!(matches!(resolved.strategy, ExecutionStrategy::External));
        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["run", "dev"]);
    });
}

#[test]
fn nci_uses_immutable_for_yarn_berry() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"yarn@4.2.0"}"#);
        fs::write(dir.path().join("yarn.lock"), "# lock").unwrap();

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nci(Vec::new(), &ctx).unwrap();

        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["install", "--immutable"]);
    });
}

#[test]
fn nci_locked_project_preserves_install_flags() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);
        fs::write(dir.path().join("package-lock.json"), "lock").unwrap();

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nci(vec!["--prefer-offline".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npm");
        assert_eq!(resolved.args, vec!["ci", "--prefer-offline"]);
    });
}

#[test]
fn nci_without_lockfile_falls_back_to_install() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"pnpm@9.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nci(Vec::new(), &ctx).unwrap();

        assert_eq!(resolved.program, "pnpm");
        assert_eq!(resolved.args, vec!["i"]);
    });
}

#[test]
fn nci_deno_lockfile_uses_frozen_install() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"deno@1.46.0"}"#);
        fs::write(dir.path().join("deno.lock"), "lock").unwrap();

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nci(Vec::new(), &ctx).unwrap();

        assert_eq!(resolved.program, "deno");
        assert_eq!(resolved.args, vec!["install", "--frozen"]);
    });
}

#[test]
fn ni_in_workspace_package_uses_root_package_manager() {
    with_skip_pm_check(|| {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("package.json"),
            r#"{"packageManager":"pnpm@9.0.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        fs::write(root.path().join("pnpm-lock.yaml"), "lock").unwrap();

        let pkg = root.path().join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        write_package_json(&pkg, r#"{"name":"app"}"#);

        let ctx = ResolveContext::new(pkg, HniConfig::default());
        let resolved = resolve::resolve_ni(vec!["vite".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "pnpm");
        assert_eq!(resolved.args, vec!["add", "vite"]);
    });
}

#[test]
fn nci_in_workspace_package_uses_root_lockfile() {
    with_skip_pm_check(|| {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("pnpm-lock.yaml"), "lock").unwrap();

        let pkg = root.path().join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        write_package_json(&pkg, r#"{"name":"app"}"#);

        let ctx = ResolveContext::new(pkg, HniConfig::default());
        let resolved = resolve::resolve_nci(Vec::new(), &ctx).unwrap();

        assert_eq!(resolved.program, "pnpm");
        assert_eq!(resolved.args, vec!["i", "--frozen-lockfile"]);
    });
}

#[test]
fn nci_in_pnpm_workspace_without_lockfile_uses_plain_install() {
    with_skip_pm_check(|| {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("package.json"),
            r#"{"packageManager":"pnpm@9.0.0","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        fs::write(
            root.path().join("pnpm-workspace.yaml"),
            "packages:\n  - packages/*\n",
        )
        .unwrap();

        let pkg = root.path().join("packages").join("app");
        fs::create_dir_all(&pkg).unwrap();
        write_package_json(&pkg, r#"{"name":"app"}"#);

        let ctx = ResolveContext::new(pkg, HniConfig::default());
        let resolved = resolve::resolve_nci(Vec::new(), &ctx).unwrap();

        assert_eq!(resolved.program, "pnpm");
        assert_eq!(resolved.args, vec!["i"]);
    });
}

#[test]
fn nlx_npm_uses_npx() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nlx(vec!["vitest".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "npx");
        assert_eq!(resolved.args, vec!["vitest"]);
    });
}

#[test]
fn nlx_fast_mode_uses_local_bin_when_present() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);
        fs::create_dir_all(dir.path().join("node_modules").join(".bin")).unwrap();
        let local_bin = dir.path().join("node_modules").join(".bin").join("vitest");
        fs::write(&local_bin, "").unwrap();
        make_executable(&local_bin);

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nlx(vec!["vitest".into(), "--help".into()], &ctx).unwrap();

        match &resolved.strategy {
            ExecutionStrategy::Native(NativeExecution::RunLocalBin(exec)) => {
                assert_eq!(exec.bin_name, "vitest");
                let expected_suffix = std::path::Path::new("node_modules")
                    .join(".bin")
                    .join("vitest");
                assert!(
                    exec.resolved_path().ends_with(&expected_suffix),
                    "expected path to end with {expected_suffix:?}, got {:?}",
                    exec.resolved_path()
                );
            }
            other => panic!("expected native local bin execution, got {other:?}"),
        }
        assert_eq!(resolved.args, vec!["--help"]);
    });
}

#[test]
fn nlx_fast_mode_falls_back_to_package_manager_when_local_bin_is_missing() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nlx(vec!["vitest".into()], &ctx).unwrap();

        assert!(matches!(resolved.strategy, ExecutionStrategy::External));
        assert_eq!(resolved.program, "npx");
        assert_eq!(
            resolved.fast_fallback_reason.as_deref(),
            Some(
                "local binary not found in node_modules/.bin or package.json bin entries; falling back to package-manager exec"
            )
        );
    });
}

#[test]
fn nlx_fast_mode_deno_remote_exec_ignores_malformed_ancestor_manifests() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("repo");
        let project = root.join("apps").join("deno-app");
        fs::create_dir_all(&project).unwrap();
        fs::write(root.join("package.json"), r#"{"name":"broken""#).unwrap();
        fs::write(project.join("deno.json"), r#"{"tasks":{"dev":"echo ok"}}"#).unwrap();

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(project, cfg);
        let resolved = resolve::resolve_nlx(vec!["create-vite".into()], &ctx).unwrap();

        assert!(matches!(resolved.strategy, ExecutionStrategy::External));
        assert_eq!(resolved.program, "deno");
        assert_eq!(resolved.args, vec!["run", "npm:create-vite"]);
    });
}

#[test]
fn nlx_fast_mode_uses_declared_package_bin_when_present() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("bin")).unwrap();
        fs::write(dir.path().join("bin").join("hello.js"), "console.log('hi')").unwrap();
        write_package_json(
            dir.path(),
            r#"{"name":"tooling","packageManager":"npm@10.0.0","bin":{"hello":"bin/hello.js"}}"#,
        );

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nlx(vec!["hello".into(), "--flag".into()], &ctx).unwrap();

        match &resolved.strategy {
            ExecutionStrategy::Native(NativeExecution::RunLocalBin(exec)) => {
                assert_eq!(exec.bin_name, "hello");
                let expected_suffix = std::path::Path::new("bin").join("hello.js");
                assert!(
                    exec.resolved_path().ends_with(&expected_suffix),
                    "expected path to end with {expected_suffix:?}, got {:?}",
                    exec.resolved_path()
                );
            }
            other => panic!("expected native local bin execution, got {other:?}"),
        }
        assert_eq!(resolved.args, vec!["--flag"]);
    });
}

#[test]
fn nlx_fast_mode_uses_pnpm_hoisted_local_bin_when_present() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"pnpm@9.0.0"}"#);
        let bin_dir = dir
            .path()
            .join("node_modules")
            .join(".pnpm")
            .join("node_modules")
            .join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        fs::write(
            bin_dir.join(if cfg!(windows) {
                "vitest.cmd"
            } else {
                "vitest"
            }),
            "",
        )
        .unwrap();
        if cfg!(unix) {
            make_executable(&bin_dir.join("vitest"));
        }

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nlx(vec!["vitest".into()], &ctx).unwrap();

        match &resolved.strategy {
            ExecutionStrategy::Native(NativeExecution::RunLocalBin(exec)) => {
                assert_eq!(exec.bin_name, "vitest");
                let bin_name = if cfg!(windows) {
                    "vitest.cmd"
                } else {
                    "vitest"
                };
                let expected_suffix = std::path::Path::new("node_modules")
                    .join(".pnpm")
                    .join("node_modules")
                    .join(".bin")
                    .join(bin_name);
                assert!(
                    exec.resolved_path().ends_with(&expected_suffix),
                    "expected path to end with {expected_suffix:?}, got {:?}",
                    exec.resolved_path()
                );
            }
            other => panic!("expected native local bin execution, got {other:?}"),
        }
    });
}

#[test]
fn nlx_fast_mode_falls_back_for_yarn_berry_pnp_declared_bin() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("bin")).unwrap();
        fs::write(dir.path().join("bin").join("hello.js"), "console.log('hi')").unwrap();
        fs::write(dir.path().join(".pnp.cjs"), "module.exports = {};\n").unwrap();
        write_package_json(
            dir.path(),
            r#"{"name":"tooling","packageManager":"yarn@4.0.0","bin":{"hello":"bin/hello.js"}}"#,
        );

        let cfg = HniConfig {
            fast_mode: true,
            ..HniConfig::default()
        };
        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_nlx(vec!["hello".into()], &ctx).unwrap();

        assert!(matches!(resolved.strategy, ExecutionStrategy::External));
        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["dlx", "hello"]);
    });
}

#[test]
fn nlx_pnpm_uses_dlx() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"pnpm@9.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nlx(vec!["vitest".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "pnpm");
        assert_eq!(resolved.args, vec!["dlx", "vitest"]);
    });
}

#[test]
fn nlx_yarn_berry_uses_dlx() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"yarn@4.0.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nlx(vec!["vitest".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "yarn");
        assert_eq!(resolved.args, vec!["dlx", "vitest"]);
    });
}

#[test]
fn nlx_bun_uses_x() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"bun@1.1.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nlx(vec!["vitest".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "bun");
        assert_eq!(resolved.args, vec!["x", "vitest"]);
    });
}

#[test]
fn nlx_deno_wraps_target_with_npm_prefix() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"deno@1.46.0"}"#);

        let ctx = ResolveContext::new(dir.path().to_path_buf(), HniConfig::default());
        let resolved = resolve::resolve_nlx(vec!["vitest".into(), "--help".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "deno");
        assert_eq!(resolved.args, vec!["run", "npm:vitest", "--help"]);
    });
}

#[test]
fn ni_global_rejects_yarn_berry_package_manager() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        write_package_json(dir.path(), r#"{"packageManager":"npm@10.0.0"}"#);

        let cfg = HniConfig {
            global_package_manager: PackageManager::YarnBerry,
            ..HniConfig::default()
        };

        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let err = resolve::resolve_ni(vec!["-g".into(), "eslint".into()], &ctx).unwrap_err();

        assert!(
            err.to_string()
                .contains("global install/uninstall is not supported by yarn (berry)")
        );
    });
}

#[test]
fn resolves_from_default_package_manager_when_no_project_hints_exist() {
    with_skip_pm_check(|| {
        let dir = tempfile::tempdir().unwrap();
        let cfg = HniConfig {
            default_package_manager: Some(PackageManager::Bun),
            ..HniConfig::default()
        };

        let ctx = ResolveContext::new(dir.path().to_path_buf(), cfg);
        let resolved = resolve::resolve_ni(vec!["vite".into()], &ctx).unwrap();

        assert_eq!(resolved.program, "bun");
        assert_eq!(resolved.args, vec!["add", "vite"]);
    });
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
