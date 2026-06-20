use std::{
    collections::HashMap,
    ffi::OsString,
    path::Path,
    process::{Command, ExitCode},
};

use anyhow::{Context, Result};
use deno_task_shell::{KillSignal, execute, parser::parse};
use tokio::{
    runtime::{Builder, Runtime},
    task::LocalSet,
};

use crate::{
    core::{
        shell::{configure_command, shell_command, shell_escape},
        types::{
            ExecutionStrategy, NativeDenoTaskExecution, NativeDenoTaskStage, NativeDenoTaskStep,
            NativeExecution, NativeLocalBinExecution, NativeLocalBinLauncher,
            NativeScriptExecution, NativeWorkspaceLocalBinExecution,
            NativeWorkspaceLocalBinPackage, NativeWorkspaceScriptExecution,
            NativeWorkspaceScriptPackage, ResolvedExecution,
        },
        util::exit_code_from_status,
    },
    platform::node::{REAL_NODE_ENV, resolve_real_node_path},
};

use super::env::{apply_local_bin_environment, native_script_env};
use super::{is_node_program, looks_like_env_assignment};

const YARN_PNP_BIN_RUNNER: &str = r#"
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const Module = require('node:module');

function fail(message) {
  console.error(message);
  process.exit(127);
}

const wanted = process.env.ALUR_PNP_BIN;
if (!wanted) fail('ALUR_PNP_BIN is required');

const issuer = process.cwd() + path.sep;
const api = Module.findPnpApi && Module.findPnpApi(issuer);
if (!api) fail(`Yarn Plug'n'Play API was not found for ${process.cwd()}`);

function packageInfo(locator) {
  try {
    return locator && api.getPackageInformation(locator);
  } catch {
    return undefined;
  }
}

function dependencyLocator(name, reference) {
  if (reference == null) return undefined;
  return typeof api.getLocator === 'function'
    ? api.getLocator(name, reference)
    : { name, reference };
}

function binPath(info, manifest, packageName) {
  const bin = manifest && manifest.bin;
  if (typeof bin === 'string') {
    const unscoped = packageName && packageName.includes('/') ? packageName.split('/').pop() : packageName;
    return packageName === wanted || unscoped === wanted ? path.join(info.packageLocation, bin) : undefined;
  }
  if (!bin || typeof bin !== 'object') return undefined;
  return typeof bin[wanted] === 'string' ? path.join(info.packageLocation, bin[wanted]) : undefined;
}

const rootLocator = api.findPackageLocator(issuer) || api.topLevel;
const queue = [rootLocator];
const rootInfo = packageInfo(rootLocator);
if (rootInfo && rootInfo.packageDependencies) {
  for (const [name, reference] of rootInfo.packageDependencies) {
    const locator = dependencyLocator(name, reference);
    if (locator) queue.push(locator);
  }
}

let script;
for (const locator of queue) {
  const info = packageInfo(locator);
  if (!info || !info.packageLocation) continue;
  const manifestPath = path.join(info.packageLocation, 'package.json');
  let manifest;
  try {
    manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  } catch {
    continue;
  }
  script = binPath(info, manifest, manifest.name || locator.name);
  if (script) break;
}

if (!script) fail(`Yarn Plug'n'Play binary '${wanted}' was not found`);

process.argv = [process.argv[0], script, ...process.argv.slice(1)];
(async () => {
  try {
    require(script);
  } catch (error) {
    if (error && error.code === 'ERR_REQUIRE_ESM') {
      await import(pathToFileURL(script).href);
    } else {
      throw error;
    }
  }
})().catch((error) => {
  console.error(error && error.stack ? error.stack : error);
  process.exit(1);
});
"#;

pub(super) fn run_script(exec: &NativeScriptExecution, invocation_cwd: &Path) -> Result<ExitCode> {
    if exec.steps.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }

    let shared_env = native_script_env(exec, invocation_cwd)?;

    for step in &exec.steps {
        let forwarded_args = if step.forward_args {
            exec.forwarded_args.as_slice()
        } else {
            &[]
        };

        let status =
            prepare_script_step_command(&step.command, forwarded_args, &exec.package_root)?
                .envs(shared_env.iter().map(|(key, value)| (key, value)))
                .env("npm_lifecycle_event", &step.event_name)
                .env("npm_lifecycle_script", &step.command)
                .status()
                .with_context(|| {
                    format!("failed to execute native script step '{}'", step.event_name)
                })?;

        if !status.success() {
            return Ok(exit_code_from_status(status.code()));
        }
    }

    Ok(ExitCode::SUCCESS)
}

pub(super) fn run_local_bin(exec: &NativeLocalBinExecution, cwd: &Path) -> Result<ExitCode> {
    let mut command = spawn_local_bin_command(exec)?;
    command.current_dir(cwd);

    apply_local_bin_environment(&mut command, exec, cwd);

    let status = command
        .status()
        .with_context(|| format!("failed to execute native local bin {}", exec.bin_name))?;

    Ok(exit_code_from_status(status.code()))
}

pub(super) fn run_workspace_scripts(
    exec: &NativeWorkspaceScriptExecution,
    invocation_cwd: &Path,
) -> Result<ExitCode> {
    let chunks = workspace_script_chunks(exec);
    for chunk in chunks {
        let code =
            run_workspace_script_chunk(&chunk, invocation_cwd, exec.concurrency, exec.stream)?;
        if code != ExitCode::SUCCESS {
            return Ok(code);
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn run_workspace_local_bins(
    exec: &NativeWorkspaceLocalBinExecution,
) -> Result<ExitCode> {
    let chunks = workspace_bin_chunks(exec);
    for chunk in chunks {
        let code = run_workspace_bin_chunk(&chunk, exec.concurrency)?;
        if code != ExitCode::SUCCESS {
            return Ok(code);
        }
    }
    Ok(ExitCode::SUCCESS)
}

pub(super) fn run_deno_task(
    exec: &NativeDenoTaskExecution,
    invocation_cwd: &Path,
) -> Result<ExitCode> {
    let envs = deno_task_env(exec, invocation_cwd)?;
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;
    let local_set = LocalSet::new();

    for stage in &exec.stages {
        for step in &stage.steps {
            print_deno_task_line(step, &exec.forwarded_args);
        }

        let Some(command) = stage_command(stage, &exec.forwarded_args) else {
            continue;
        };

        let exit_code =
            execute_deno_shell_command(&runtime, &local_set, &command, &exec.project_root, &envs)?;
        if exit_code != 0 {
            return Ok(exit_code_from_status(Some(exit_code)));
        }
    }

    Ok(ExitCode::SUCCESS)
}

pub(super) fn format_command(exec: &ResolvedExecution) -> String {
    match &exec.strategy {
        ExecutionStrategy::Native(NativeExecution::RunScript(native)) => {
            let mut rendered = vec![
                "alur".to_string(),
                "fast:run-script".to_string(),
                native.script_name.clone(),
            ];
            rendered.extend(native.forwarded_args.clone());
            join_rendered(&rendered)
        }
        ExecutionStrategy::Native(NativeExecution::RunDenoTask(native)) => {
            let mut rendered = vec![
                "alur".to_string(),
                "fast:run-deno-task".to_string(),
                native.selection.clone(),
            ];
            if !native.forwarded_args.is_empty() {
                rendered.push("--".to_string());
                rendered.extend(native.forwarded_args.clone());
            }
            join_rendered(&rendered)
        }
        ExecutionStrategy::Native(NativeExecution::RunLocalBin(native)) => {
            let mut rendered = vec![
                "alur".to_string(),
                "fast:run-local-bin".to_string(),
                native.bin_name.clone(),
            ];
            rendered.extend(native.forwarded_args.clone());
            join_rendered(&rendered)
        }
        ExecutionStrategy::Native(NativeExecution::RunWorkspaceScripts(native)) => {
            join_rendered(&[
                "alur".to_string(),
                "fast:run-workspace-script".to_string(),
                native.script_name.clone(),
            ])
        }
        ExecutionStrategy::Native(NativeExecution::RunWorkspaceLocalBins(native)) => {
            join_rendered(&[
                "alur".to_string(),
                "fast:run-workspace-bin".to_string(),
                native.bin_name.clone(),
            ])
        }
        ExecutionStrategy::External | ExecutionStrategy::InternalBatch { .. } => String::new(),
    }
}

fn join_rendered(rendered: &[String]) -> String {
    rendered
        .iter()
        .map(|part| shell_escape(part))
        .collect::<Vec<_>>()
        .join(" ")
}

fn deno_task_env(
    exec: &NativeDenoTaskExecution,
    invocation_cwd: &Path,
) -> Result<HashMap<OsString, OsString>> {
    let mut envs = std::env::vars_os().collect::<HashMap<_, _>>();
    envs.insert(
        OsString::from("INIT_CWD"),
        invocation_cwd.as_os_str().to_os_string(),
    );
    envs.insert(
        OsString::from("PATH"),
        OsString::from(super::env::merged_path_with_bins(&exec.bin_paths)?),
    );

    if let Ok(real_node) = resolve_real_node_path() {
        envs.insert(OsString::from(REAL_NODE_ENV), real_node.into_os_string());
    }

    Ok(envs)
}

fn stage_command(stage: &NativeDenoTaskStage, forwarded_args: &[String]) -> Option<String> {
    if stage.steps.is_empty() {
        return None;
    }

    let commands = stage
        .steps
        .iter()
        .map(|step| {
            let command = deno_task_command_string(
                &step.command,
                if step.forward_args {
                    forwarded_args
                } else {
                    &[]
                },
            );
            if stage.steps.len() == 1 {
                command
            } else {
                format!("({command})")
            }
        })
        .collect::<Vec<_>>();

    Some(if commands.len() == 1 {
        commands[0].clone()
    } else {
        commands.join(" & ")
    })
}

fn deno_task_command_string(command: &str, forwarded_args: &[String]) -> String {
    if forwarded_args.is_empty() {
        return command.to_string();
    }

    format!(
        "{command} -- {}",
        forwarded_args
            .iter()
            .map(|arg| shell_escape(arg))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn print_deno_task_line(step: &NativeDenoTaskStep, forwarded_args: &[String]) {
    if step.forward_args && !forwarded_args.is_empty() {
        println!(
            "Task {} {} -- {}",
            step.task_name,
            step.command,
            forwarded_args
                .iter()
                .map(|arg| shell_escape(arg))
                .collect::<Vec<_>>()
                .join(" ")
        );
    } else {
        println!("Task {} {}", step.task_name, step.command);
    }
}

fn execute_deno_shell_command(
    runtime: &Runtime,
    local_set: &LocalSet,
    command: &str,
    cwd: &Path,
    envs: &HashMap<OsString, OsString>,
) -> Result<i32> {
    let parsed = parse(command).context("failed to parse deno task command")?;

    Ok(runtime.block_on(local_set.run_until(execute(
        parsed,
        envs.clone(),
        cwd.to_path_buf(),
        HashMap::default(),
        KillSignal::default(),
    ))))
}

fn workspace_script_chunks(
    exec: &NativeWorkspaceScriptExecution,
) -> Vec<Vec<NativeWorkspaceScriptPackage>> {
    if exec.parallel {
        vec![exec.chunks.iter().flatten().cloned().collect()]
    } else {
        exec.chunks.clone()
    }
}

fn workspace_bin_chunks(
    exec: &NativeWorkspaceLocalBinExecution,
) -> Vec<Vec<NativeWorkspaceLocalBinPackage>> {
    if exec.parallel {
        vec![exec.chunks.iter().flatten().cloned().collect()]
    } else {
        exec.chunks.clone()
    }
}

fn run_workspace_script_chunk(
    packages: &[NativeWorkspaceScriptPackage],
    invocation_cwd: &Path,
    concurrency: usize,
    stream: bool,
) -> Result<ExitCode> {
    run_bounded(packages, concurrency, |package| {
        if stream {
            eprintln!("{} {}", package.package_name, package.exec.script_name);
        }
        run_script(&package.exec, invocation_cwd)
    })
}

fn run_workspace_bin_chunk(
    packages: &[NativeWorkspaceLocalBinPackage],
    concurrency: usize,
) -> Result<ExitCode> {
    run_bounded(packages, concurrency, |package| {
        run_local_bin(&package.exec, &package.package_root)
    })
}

fn run_bounded<T, F>(items: &[T], concurrency: usize, run_one: F) -> Result<ExitCode>
where
    T: Sync,
    F: Fn(&T) -> Result<ExitCode> + Sync,
{
    let concurrency = concurrency.max(1);
    let mut first_failure = ExitCode::SUCCESS;

    for batch in items.chunks(concurrency) {
        let results = std::thread::scope(|scope| {
            let handles = batch
                .iter()
                .map(|item| scope.spawn(|| run_one(item)))
                .collect::<Vec<_>>();

            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .map_err(|_| anyhow::anyhow!("workspace worker panicked"))?
                })
                .collect::<Result<Vec<_>>>()
        })?;

        for code in results {
            if code != ExitCode::SUCCESS && first_failure == ExitCode::SUCCESS {
                first_failure = code;
            }
        }
    }

    Ok(first_failure)
}

fn spawn_local_bin_command(exec: &NativeLocalBinExecution) -> Result<Command> {
    let mut command = match &exec.launcher {
        NativeLocalBinLauncher::Binary(path) => Command::new(path),
        NativeLocalBinLauncher::Cmd(path) => {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(path);
            command
        }
        NativeLocalBinLauncher::PowerShell(path) => {
            let mut command = Command::new("powershell");
            command
                .arg("-NoProfile")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-File")
                .arg(path);
            command
        }
        NativeLocalBinLauncher::NodeScript {
            script_path,
            node_args,
        } => {
            let mut command = Command::new(resolve_real_node_path()?);
            command.args(node_args).arg(script_path);
            command
        }
        NativeLocalBinLauncher::YarnPnp {
            pnp_loader,
            bin_name,
        } => {
            let mut command = Command::new(resolve_real_node_path()?);
            command
                .arg("-r")
                .arg(pnp_loader)
                .arg("-e")
                .arg(YARN_PNP_BIN_RUNNER)
                .env("ALUR_PNP_BIN", bin_name);
            command
        }
    };

    command.args(&exec.forwarded_args);
    Ok(configure_command(command, Path::new(".")))
}

fn prepare_script_step_command(
    command_string: &str,
    forwarded_args: &[String],
    cwd: &Path,
) -> Result<Command> {
    if let Some(command) = spawn_direct_script_command(command_string, forwarded_args)? {
        return Ok(configure_command(command, cwd));
    }

    Ok(spawn_shell_command(command_string, forwarded_args, cwd))
}

fn spawn_direct_script_command(
    command_string: &str,
    forwarded_args: &[String],
) -> Result<Option<Command>> {
    if cfg!(windows) || contains_shell_metacharacters(command_string) {
        return Ok(None);
    }

    let Some(tokens) = shlex::split(command_string) else {
        return Ok(None);
    };
    let Some((program, args)) = tokens.split_first() else {
        return Ok(None);
    };

    if looks_like_env_assignment(program) {
        return Ok(None);
    }

    let mut command = if is_node_program(program) {
        Command::new(resolve_real_node_path()?)
    } else {
        Command::new(program)
    };
    command.args(args).args(forwarded_args);
    Ok(Some(command))
}

fn spawn_shell_command(command_string: &str, forwarded_args: &[String], cwd: &Path) -> Command {
    let line = build_shell_command_line(command_string, forwarded_args);
    configure_command(shell_command(&line), cwd)
}

fn build_shell_command_line(command_string: &str, forwarded_args: &[String]) -> String {
    if forwarded_args.is_empty() {
        return command_string.to_string();
    }

    let escaped_args = forwarded_args
        .iter()
        .map(|arg| shell_arg_escape(arg))
        .collect::<Vec<_>>()
        .join(" ");

    format!("{command_string} {escaped_args}")
}

#[cfg(not(windows))]
fn shell_arg_escape(arg: &str) -> String {
    shell_escape(arg)
}

#[cfg(windows)]
fn shell_arg_escape(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_string();
    }

    if arg.chars().all(|ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '=' | '@' | '\\')
    }) {
        return arg.to_string();
    }

    format!("\"{}\"", arg.replace('"', "\"\""))
}

fn contains_shell_metacharacters(command_string: &str) -> bool {
    command_string.chars().any(|ch| {
        matches!(
            ch,
            '|' | '&'
                | ';'
                | '<'
                | '>'
                | '('
                | ')'
                | '$'
                | '`'
                | '*'
                | '?'
                | '['
                | ']'
                | '{'
                | '}'
                | '~'
                | '\n'
                | '\r'
        )
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::core::types::NativeScriptStep;

    #[test]
    fn glob_patterns_force_shell_fallback() {
        assert!(contains_shell_metacharacters("printf \"%s\\n\" src/*.js"));
        assert!(
            spawn_direct_script_command("printf \"%s\\n\" src/*.js", &[])
                .unwrap()
                .is_none()
        );
    }

    #[cfg(windows)]
    #[test]
    fn direct_script_fast_path_is_disabled_on_windows() {
        assert!(
            spawn_direct_script_command(r"node .\\scripts\\build.js", &[])
                .unwrap()
                .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn shell_fallback_does_not_leak_forwarded_args_into_shell_positionals() {
        let dir = tempfile::tempdir().unwrap();
        let package_json_path = dir.path().join("package.json");
        fs::write(&package_json_path, "{}").unwrap();
        let output_path = dir.path().join("count.txt");
        let capture_script = dir.path().join("capture.sh");
        fs::write(
            &capture_script,
            format!(
                "#!/bin/sh\nprintf '%s' \"$#\" > {}\n",
                shell_escape(output_path.to_str().unwrap())
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mut perms = fs::metadata(&capture_script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&capture_script, perms).unwrap();
        }
        let command = format!(
            "{} \"$@\" && :",
            shell_escape(capture_script.to_str().unwrap())
        );

        let exec = NativeScriptExecution {
            package_root: dir.path().to_path_buf(),
            package_json_path,
            script_name: "count".to_string(),
            steps: vec![NativeScriptStep {
                event_name: "count".to_string(),
                command,
                forward_args: true,
            }],
            forwarded_args: vec!["alpha".to_string(), "two words".to_string()],
            bin_paths: Vec::new(),
        };

        run_script(&exec, dir.path()).unwrap();
        assert_eq!(fs::read_to_string(output_path).unwrap(), "0");
    }
}
