use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
};

use anyhow::{Context, Result, anyhow};

use crate::{
    core::{
        project::{parse_package_manager_field, read_nearest_package_json_path},
        resolve::{ResolveContext, execute_command},
        shell::shell_escape,
        types::PackageManager,
        util::exit_code_from_status,
    },
    platform::paths_equal,
};

const PM_SHIM_NAMES: &[&str] = &[
    "npm", "npx", "pnpm", "pnpx", "yarn", "yarnpkg", "bun", "bunx", "deno",
];

pub fn run(args: Vec<String>, ctx: &ResolveContext) -> Result<ExitCode> {
    match args.first().map(String::as_str) {
        Some("which") => {
            pm_which(ctx)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("use") => {
            let spec = args
                .get(1)
                .ok_or_else(|| anyhow!("parse error: pm use requires a package manager"))?;
            pm_use(ctx.cwd(), spec)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("shim") => {
            pm_shim()?;
            Ok(ExitCode::SUCCESS)
        }
        Some("run") => {
            let invoked = args
                .get(1)
                .ok_or_else(|| anyhow!("parse error: pm run requires an invoked command"))?;
            pm_run(ctx, invoked, &args[2..])
        }
        Some(other) => Err(anyhow!(
            "parse error: unknown pm command '{other}'. Try: alur pm which"
        )),
        None => Err(anyhow!(
            "parse error: pm requires a command. Try: alur pm which"
        )),
    }
}

fn pm_which(ctx: &ResolveContext) -> Result<()> {
    let detection = ctx.detect()?;
    let pm = detection
        .agent
        .ok_or_else(|| anyhow!("no package manager detected"))?;
    let path = which::which(pm.bin()).unwrap_or_else(|_| pm.bin().into());
    println!("{}", path.display());
    eprintln!("resolved from {:?}", detection.source);
    Ok(())
}

fn pm_use(cwd: &Path, spec: &str) -> Result<()> {
    let (pm, _) = parse_package_manager_field(spec)
        .ok_or_else(|| anyhow!("unsupported package manager spec '{spec}'"))?;
    let path = read_nearest_package_json_path(cwd).ok_or_else(|| {
        anyhow!("pm use requires a package.json at or above the current directory")
    })?;
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut json = serde_json::from_str::<serde_json::Value>(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let object = json
        .as_object_mut()
        .ok_or_else(|| anyhow!("{} must contain a JSON object", path.display()))?;
    object.insert(
        "packageManager".to_string(),
        serde_json::Value::String(spec.to_string()),
    );
    let rendered = serde_json::to_string_pretty(&json)?;
    fs::write(&path, format!("{rendered}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    println!("packageManager: {} ({})", spec, pm.display_name());
    Ok(())
}

fn pm_shim() -> Result<()> {
    let dir = pm_shim_dir()?;
    let exe_path = current_binary_path()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create pm shim directory at {}", dir.display()))?;

    for name in PM_SHIM_NAMES {
        let path = dir.join(pm_shim_file_name(name));
        write_pm_shim(&path, &exe_path, name)
            .with_context(|| format!("failed to write pm shim {}", path.display()))?;
    }

    println!("created package-manager shims in {}", dir.display());
    println!("add this directory before package managers on PATH:");
    if cfg!(windows) {
        println!("  $env:PATH = '{};' + $env:PATH", dir.display());
    } else {
        println!(
            "  export PATH={}:$PATH",
            shell_escape(dir.to_string_lossy().as_ref())
        );
    }
    Ok(())
}

fn pm_run(ctx: &ResolveContext, invoked: &str, args: &[String]) -> Result<ExitCode> {
    let detection = ctx.detect()?;
    let target_pm = detection
        .agent
        .or_else(|| package_manager_from_invocation(invoked))
        .ok_or_else(|| anyhow!("no package manager detected"))?;

    let (program, command_args) = if is_exec_invocation(invoked) {
        execute_command(target_pm, args.to_vec())
    } else {
        (target_pm.bin().to_string(), args.to_vec())
    };

    let mut command = Command::new(&program);
    command
        .args(command_args)
        .current_dir(ctx.cwd())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(path) = path_without_pm_shims(env::var_os("PATH"))? {
        command.env("PATH", path);
    }

    let status = command
        .status()
        .with_context(|| format!("failed to execute {program}"))?;
    Ok(exit_code_from_status(status.code()))
}

fn pm_shim_dir() -> Result<PathBuf> {
    if let Some(path) = env::var_os("ALUR_PM_SHIM_DIR").filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    dirs::data_local_dir()
        .or_else(dirs::config_dir)
        .map(|dir| dir.join("alur").join("pm-shims"))
        .ok_or_else(|| anyhow!("unable to determine managed alur pm shim directory"))
}

fn current_binary_path() -> Result<PathBuf> {
    let exe_path = env::current_exe().context("failed to determine current executable path")?;
    Ok(dunce::canonicalize(&exe_path).unwrap_or(exe_path))
}

fn pm_shim_file_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.cmd")
    } else {
        name.to_string()
    }
}

fn write_pm_shim(path: &Path, exe_path: &Path, name: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let exe = exe_path.to_string_lossy().replace('"', "\"\"");
        fs::write(path, format!("@echo off\r\n\"{exe}\" pm run {name} %*\r\n"))
    }

    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;

        fs::write(
            path,
            format!(
                "#!/bin/sh\nexec {} pm run {} \"$@\"\n",
                shell_escape(exe_path.to_string_lossy().as_ref()),
                shell_escape(name)
            ),
        )?;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
    }
}

fn path_without_pm_shims(path: Option<OsString>) -> Result<Option<OsString>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let shim_dir = pm_shim_dir()?;
    let entries = env::split_paths(&path)
        .filter(|entry| !paths_equal(entry, &shim_dir))
        .collect::<Vec<_>>();
    env::join_paths(entries)
        .map(Some)
        .map_err(|error| anyhow!("failed to rebuild PATH without pm shims: {error}"))
}

fn package_manager_from_invocation(invoked: &str) -> Option<PackageManager> {
    match normalized_invocation(invoked).as_str() {
        "npm" | "npx" => Some(PackageManager::Npm),
        "pnpm" | "pnpx" => Some(PackageManager::Pnpm),
        "yarn" | "yarnpkg" => Some(PackageManager::Yarn),
        "bun" | "bunx" => Some(PackageManager::Bun),
        "deno" => Some(PackageManager::Deno),
        other => PackageManager::from_name(other),
    }
}

fn is_exec_invocation(invoked: &str) -> bool {
    matches!(
        normalized_invocation(invoked).as_str(),
        "npx" | "pnpx" | "bunx"
    )
}

fn normalized_invocation(invoked: &str) -> String {
    let lower = invoked.trim().to_ascii_lowercase();
    lower
        .trim_end_matches(".cmd")
        .trim_end_matches(".exe")
        .to_string()
}
