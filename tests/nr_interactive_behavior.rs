mod support;

#[test]
fn run_custom_completion_query_flag_is_treated_as_a_script_name() {
    support::with_env_lock(|| {
        let work = tempfile::tempdir().unwrap();
        let project = work.path().join("npm");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("package-lock.json"), "lock").unwrap();
        std::fs::write(
            project.join("package.json"),
            r#"{"name":"x","scripts":{"dev":"vite"}}"#,
        )
        .unwrap();

        let output = support::run_alur(
            vec![
                "run",
                "-C",
                project.to_str().unwrap(),
                "--pm",
                "--print-command",
                "--completion",
            ],
            &[("ALUR_SKIP_PM_CHECK", "1")],
        );

        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "npm run --completion"
        );
    });
}
