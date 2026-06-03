use crate::core::types::PackageManager;

#[must_use]
pub fn exclude_flag(mut args: Vec<String>, flag: &str) -> Vec<String> {
    if let Some(pos) = args.iter().position(|arg| arg == flag) {
        args.remove(pos);
    }
    args
}

pub(super) fn normalize_ni_args(args: Vec<String>, pm: PackageManager) -> Vec<String> {
    args.into_iter()
        .map(|arg| match arg.as_str() {
            "-D" if pm == PackageManager::Bun => "-d".to_string(),
            "-P" if pm == PackageManager::Npm => "--omit=dev".to_string(),
            "-P" => "--production".to_string(),
            _ => arg,
        })
        .collect()
}

pub(super) fn npm_run_args(args: Vec<String>) -> Vec<String> {
    if args.len() <= 1 {
        return prepend("run", args);
    }

    let mut out = vec!["run".to_string(), args[0].clone(), "--".to_string()];
    out.extend(args.into_iter().skip(1));
    out
}

pub(super) fn prepend(head: &str, mut tail: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(tail.len() + 1);
    out.push(head.to_string());
    out.append(&mut tail);
    out
}
