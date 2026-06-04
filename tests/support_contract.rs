mod support;

#[test]
fn with_var_removed_restores_absent_state() {
    support::with_env_lock(|| {
        support::remove_var("ALUR_TEST_TMP_VAR");
        support::with_var_removed("ALUR_TEST_TMP_VAR", || {
            support::set_var("ALUR_TEST_TMP_VAR", "leaked");
        });
        assert!(std::env::var_os("ALUR_TEST_TMP_VAR").is_none());
    });
}

#[test]
fn with_var_removed_restores_existing_value() {
    support::with_env_lock(|| {
        support::set_var("ALUR_TEST_TMP_VAR", "before");
        support::with_var_removed("ALUR_TEST_TMP_VAR", || {
            support::set_var("ALUR_TEST_TMP_VAR", "during");
        });
        assert_eq!(
            std::env::var("ALUR_TEST_TMP_VAR").ok().as_deref(),
            Some("before")
        );
        support::remove_var("ALUR_TEST_TMP_VAR");
    });
}
