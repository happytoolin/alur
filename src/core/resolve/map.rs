use crate::core::types::PackageManager;

use super::flags::{npm_run_args, prepend};

#[must_use]
pub fn version_command_for_pm(pm: PackageManager) -> (String, Vec<String>) {
    (pm.bin().to_string(), vec!["--version".to_string()])
}

pub(super) fn install_command(pm: PackageManager, args: Vec<String>) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm => ("npm".to_string(), prepend("i", args)),
        PackageManager::Yarn | PackageManager::YarnBerry => {
            ("yarn".to_string(), prepend("install", args))
        }
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("i", args)),
        PackageManager::Bun => ("bun".to_string(), prepend("install", args)),
        PackageManager::Deno => ("deno".to_string(), prepend("install", args)),
    }
}

pub(super) fn add_command(pm: PackageManager, args: Vec<String>) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm => ("npm".to_string(), prepend("i", args)),
        PackageManager::Yarn | PackageManager::YarnBerry => {
            ("yarn".to_string(), prepend("add", args))
        }
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("add", args)),
        PackageManager::Bun => ("bun".to_string(), prepend("add", args)),
        PackageManager::Deno => ("deno".to_string(), prepend("add", args)),
    }
}

pub(super) fn run_command(pm: PackageManager, args: Vec<String>) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm => ("npm".to_string(), npm_run_args(args)),
        PackageManager::Yarn | PackageManager::YarnBerry => {
            ("yarn".to_string(), prepend("run", args))
        }
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("run", args)),
        PackageManager::Bun => ("bun".to_string(), prepend("run", args)),
        PackageManager::Deno => ("deno".to_string(), prepend("task", args)),
    }
}

pub(super) fn execute_command(pm: PackageManager, args: Vec<String>) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm | PackageManager::Yarn => ("npx".to_string(), args),
        PackageManager::YarnBerry => ("yarn".to_string(), prepend("dlx", args)),
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("dlx", args)),
        PackageManager::Bun => ("bun".to_string(), prepend("x", args)),
        PackageManager::Deno => {
            if let Some((first, rest)) = args.split_first() {
                let mut deno_args = vec!["run".to_string(), format!("npm:{first}")];
                deno_args.extend(rest.iter().cloned());
                ("deno".to_string(), deno_args)
            } else {
                ("deno".to_string(), vec!["run".to_string()])
            }
        }
    }
}

pub(super) fn upgrade_command(pm: PackageManager, args: Vec<String>) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm => ("npm".to_string(), prepend("update", args)),
        PackageManager::Yarn => ("yarn".to_string(), prepend("upgrade", args)),
        PackageManager::YarnBerry => ("yarn".to_string(), prepend("up", args)),
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("update", args)),
        PackageManager::Bun => ("bun".to_string(), prepend("update", args)),
        PackageManager::Deno => (
            "deno".to_string(),
            prepend("outdated", prepend("--update", args)),
        ),
    }
}

pub(super) fn uninstall_command(pm: PackageManager, args: Vec<String>) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm => ("npm".to_string(), prepend("uninstall", args)),
        PackageManager::Yarn | PackageManager::YarnBerry => {
            ("yarn".to_string(), prepend("remove", args))
        }
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("remove", args)),
        PackageManager::Bun => ("bun".to_string(), prepend("remove", args)),
        PackageManager::Deno => ("deno".to_string(), prepend("remove", args)),
    }
}

pub(super) fn frozen_command(pm: PackageManager, args: Vec<String>) -> (String, Vec<String>) {
    let (program, mut frozen_args) = match pm {
        PackageManager::Npm => ("npm".to_string(), vec!["ci".to_string()]),
        PackageManager::Yarn => (
            "yarn".to_string(),
            vec!["install".to_string(), "--frozen-lockfile".to_string()],
        ),
        PackageManager::YarnBerry => (
            "yarn".to_string(),
            vec!["install".to_string(), "--immutable".to_string()],
        ),
        PackageManager::Pnpm => (
            "pnpm".to_string(),
            vec!["i".to_string(), "--frozen-lockfile".to_string()],
        ),
        PackageManager::Bun => (
            "bun".to_string(),
            vec!["install".to_string(), "--frozen-lockfile".to_string()],
        ),
        PackageManager::Deno => (
            "deno".to_string(),
            vec!["install".to_string(), "--frozen".to_string()],
        ),
    };

    frozen_args.extend(args);
    (program, frozen_args)
}

pub(super) fn global_install_command(
    pm: PackageManager,
    args: Vec<String>,
) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm => ("npm".to_string(), prepend("i", prepend("-g", args))),
        PackageManager::Yarn | PackageManager::YarnBerry => yarn_global_command("add", args),
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("add", prepend("-g", args))),
        PackageManager::Bun => ("bun".to_string(), prepend("add", prepend("-g", args))),
        PackageManager::Deno => ("deno".to_string(), prepend("install", prepend("-g", args))),
    }
}

pub(super) fn global_uninstall_command(
    pm: PackageManager,
    args: Vec<String>,
) -> (String, Vec<String>) {
    match pm {
        PackageManager::Npm => ("npm".to_string(), prepend("uninstall", prepend("-g", args))),
        PackageManager::Yarn | PackageManager::YarnBerry => yarn_global_command("remove", args),
        PackageManager::Pnpm => ("pnpm".to_string(), prepend("remove", prepend("-g", args))),
        PackageManager::Bun => ("bun".to_string(), prepend("remove", prepend("-g", args))),
        PackageManager::Deno => (
            "deno".to_string(),
            prepend("uninstall", prepend("-g", args)),
        ),
    }
}

fn yarn_global_command(action: &str, args: Vec<String>) -> (String, Vec<String>) {
    let mut out = vec!["global".to_string(), action.to_string()];
    out.extend(args);
    ("yarn".to_string(), out)
}
