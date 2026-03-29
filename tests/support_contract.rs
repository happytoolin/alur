mod support;

#[test]
fn with_var_removed_restores_absent_state() {
    support::with_env_lock(|| {
        support::remove_var("HNI_TEST_TMP_VAR");
        support::with_var_removed("HNI_TEST_TMP_VAR", || {
            support::set_var("HNI_TEST_TMP_VAR", "leaked");
        });
        assert!(std::env::var_os("HNI_TEST_TMP_VAR").is_none());
    });
}

#[test]
fn with_var_removed_restores_existing_value() {
    support::with_env_lock(|| {
        support::set_var("HNI_TEST_TMP_VAR", "before");
        support::with_var_removed("HNI_TEST_TMP_VAR", || {
            support::set_var("HNI_TEST_TMP_VAR", "during");
        });
        assert_eq!(
            std::env::var("HNI_TEST_TMP_VAR").ok().as_deref(),
            Some("before")
        );
        support::remove_var("HNI_TEST_TMP_VAR");
    });
}
