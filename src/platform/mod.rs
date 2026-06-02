pub mod node;

use std::path::Path;

#[must_use]
pub fn paths_equal(a: &Path, b: &Path) -> bool {
    a == b
        || dunce::canonicalize(a)
            .ok()
            .zip(dunce::canonicalize(b).ok())
            .is_some_and(|(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::paths_equal;

    #[test]
    fn paths_equal_matches_identical_paths_without_canonicalization() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing");

        assert!(paths_equal(&path, &path));
    }

    #[cfg(unix)]
    #[test]
    fn paths_equal_matches_canonicalized_symlinks() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        std::fs::write(&target, "content").unwrap();
        symlink(&target, &link).unwrap();

        assert!(paths_equal(&target, &link));
    }
}
