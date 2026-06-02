use std::{path::Path, process::ExitCode};

#[cfg(unix)]
pub fn file_is_runnable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
pub fn file_is_runnable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
pub fn has_unix_executable_bit(path: &Path) -> bool {
    file_is_runnable(path)
}

#[cfg(not(unix))]
pub fn has_unix_executable_bit(_path: &Path) -> bool {
    false
}

pub fn exit_code_from_status(code: Option<i32>) -> ExitCode {
    code.map_or_else(|| ExitCode::from(1), exit_code_from_code)
}

pub fn exit_code_from_code(code: i32) -> ExitCode {
    let code = u8::try_from(code).unwrap_or(1);
    ExitCode::from(code)
}
