use std::{path::Path, process::ExitCode};

use is_executable::IsExecutable;

pub fn file_is_runnable(path: &Path) -> bool {
    path.is_executable()
}

pub fn exit_code_from_status(code: Option<i32>) -> ExitCode {
    code.map_or_else(|| ExitCode::from(1), exit_code_from_code)
}

pub fn exit_code_from_code(code: i32) -> ExitCode {
    let code = u8::try_from(code).unwrap_or(1);
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::{exit_code_from_code, exit_code_from_status, file_is_runnable};

    #[test]
    fn exit_code_helpers_normalize_missing_and_out_of_range_values() {
        assert_eq!(exit_code_from_status(None), std::process::ExitCode::from(1));
        assert_eq!(
            exit_code_from_status(Some(7)),
            std::process::ExitCode::from(7)
        );
        assert_eq!(exit_code_from_code(300), std::process::ExitCode::from(1));
    }

    #[cfg(unix)]
    #[test]
    fn file_is_runnable_requires_file_with_executable_bit() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("tool");
        std::fs::write(&file, "#!/bin/sh\n").unwrap();

        assert!(!file_is_runnable(&file));

        let mut permissions = std::fs::metadata(&file).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&file, permissions).unwrap();

        assert!(file_is_runnable(&file));
        assert!(!file_is_runnable(dir.path()));
    }
}
