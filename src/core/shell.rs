use std::{
    path::Path,
    process::{Command, Stdio},
};

#[cfg(windows)]
pub fn shell_command(command_string: &str) -> Command {
    use std::os::windows::process::CommandExt;

    let mut cmd = Command::new("cmd");
    // Pass the shell payload through without Rust's Windows argument quoting
    // so cmd.exe can interpret redirection and control operators correctly.
    cmd.raw_arg("/C").raw_arg(command_string);
    cmd
}

#[cfg(not(windows))]
pub fn shell_command(command_string: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", command_string]);
    cmd
}

pub fn configure_command(mut command: Command, cwd: &Path) -> Command {
    command
        .current_dir(cwd)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    command
}

pub fn shell_escape(input: &str) -> String {
    if input
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '=' | '@'))
    {
        return input.to_string();
    }

    let double_quoted = format!("\"{}\"", input.replace('"', "\\\""));
    if shlex::split(&double_quoted).is_some_and(|parts| parts.len() == 1 && parts[0] == input) {
        return double_quoted;
    }

    shlex::try_quote(input)
        .map(std::borrow::Cow::into_owned)
        .unwrap_or(double_quoted)
}

#[cfg(test)]
mod tests {
    use super::{shell_command, shell_escape};

    #[test]
    fn shell_escape_leaves_safe_tokens_unquoted() {
        assert_eq!(
            shell_escape("npm-package/bin:test=value@1.0"),
            "npm-package/bin:test=value@1.0"
        );
    }

    #[test]
    fn shell_escape_quotes_shell_sensitive_tokens() {
        assert_eq!(shell_escape("hello world"), "\"hello world\"");
        assert_eq!(shell_escape("say \"hi\""), "\"say \\\"hi\\\"\"");
    }

    #[test]
    fn shell_command_uses_platform_shell() {
        let command = shell_command("echo hi");

        #[cfg(windows)]
        assert_eq!(command.get_program(), "cmd");

        #[cfg(not(windows))]
        {
            assert_eq!(command.get_program(), "sh");
            assert_eq!(
                command
                    .get_args()
                    .map(std::ffi::OsStr::to_string_lossy)
                    .collect::<Vec<_>>(),
                ["-c", "echo hi"]
            );
        }
    }
}
