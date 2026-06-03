use std::path::{Path, PathBuf};

use super::{is_node_program, looks_like_env_assignment};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NodeBinLaunch {
    pub script_path: PathBuf,
    pub node_args: Vec<String>,
}

pub(super) fn node_args_from_shebang(line: &str) -> Option<Vec<String>> {
    let shebang = line.strip_prefix("#!")?.trim();
    let mut tokens = shlex::split(shebang)?;
    if tokens.is_empty() {
        return None;
    }

    if is_env_program(&tokens[0]) {
        tokens.remove(0);
        if tokens.first().is_some_and(|token| token == "-S") {
            tokens.remove(0);
        }
        while tokens
            .first()
            .is_some_and(|token| looks_like_env_assignment(token))
        {
            tokens.remove(0);
        }
    }

    let program = tokens.first()?;
    if !is_node_program(program) {
        return None;
    }

    Some(tokens.into_iter().skip(1).collect())
}

pub(super) fn parse_node_shell_shim(raw: &str, shim_path: &Path) -> Option<NodeBinLaunch> {
    let shim_dir = shim_path.parent()?;

    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("exec ") {
            continue;
        }

        let Some(tokens) = shlex::split(trimmed) else {
            continue;
        };

        if tokens.len() < 4 || tokens.first().map(String::as_str) != Some("exec") {
            continue;
        }

        let Some(program) = tokens.get(1) else {
            continue;
        };
        if !(is_node_program(program) || is_basedir_node_program(program)) {
            continue;
        }

        if tokens.last().map(String::as_str) != Some("$@") {
            continue;
        }

        let Some(script_token) = tokens.get(tokens.len() - 2) else {
            continue;
        };
        let Some(script_path) = resolve_shim_path_token(script_token, shim_dir) else {
            continue;
        };
        if !looks_like_node_script_path(&script_path) {
            continue;
        }

        return Some(NodeBinLaunch {
            script_path,
            node_args: tokens[2..tokens.len() - 2].to_vec(),
        });
    }

    None
}

fn resolve_shim_path_token(token: &str, shim_dir: &Path) -> Option<PathBuf> {
    if let Some(relative) = token.strip_prefix("$basedir/") {
        return Some(shim_dir.join(relative));
    }

    if let Some(relative) = token.strip_prefix("$basedir\\") {
        return Some(shim_dir.join(relative.replace('\\', "/")));
    }

    if !token.contains('/') && !token.contains('\\') {
        return None;
    }

    let path = Path::new(token);
    Some(if path.is_absolute() {
        path.to_path_buf()
    } else {
        shim_dir.join(path)
    })
}

fn looks_like_node_script_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(std::ffi::OsStr::to_str)
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("js" | "cjs" | "mjs")
    )
}

fn is_env_program(program: &str) -> bool {
    Path::new(program)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("env"))
}

fn is_basedir_node_program(program: &str) -> bool {
    matches!(program, "$basedir/node" | "$basedir/node.exe")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_node_shebang_with_env_and_args() {
        let args =
            node_args_from_shebang("#!/usr/bin/env -S node --no-warnings --trace-deprecation")
                .unwrap();
        assert_eq!(args, vec!["--no-warnings", "--trace-deprecation"]);
    }

    #[test]
    fn ignores_non_node_shebangs() {
        assert_eq!(node_args_from_shebang("#!/usr/bin/env bash"), None);
    }

    #[test]
    fn parses_npm_style_shell_shim_fixture() {
        let dir = tempfile::tempdir().unwrap();
        let shim = dir.path().join("node_modules").join(".bin").join("hello");
        let parsed = parse_node_shell_shim(
            include_str!("../../../tests/fixtures/native/npm-shim.sh"),
            &shim,
        )
        .unwrap();

        assert_eq!(
            parsed,
            NodeBinLaunch {
                script_path: shim.parent().unwrap().join("../hello/cli.js"),
                node_args: vec!["--no-warnings".to_string()],
            }
        );
    }

    #[test]
    fn parses_yarn_style_shell_shim_fixture() {
        let dir = tempfile::tempdir().unwrap();
        let shim = dir.path().join("node_modules").join(".bin").join("tool");
        let parsed = parse_node_shell_shim(
            include_str!("../../../tests/fixtures/native/yarn-shim.sh"),
            &shim,
        )
        .unwrap();

        assert_eq!(
            parsed,
            NodeBinLaunch {
                script_path: shim.parent().unwrap().join("../tool/bin.mjs"),
                node_args: Vec::new(),
            }
        );
    }
}
