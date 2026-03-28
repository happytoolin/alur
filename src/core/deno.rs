use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use indexmap::IndexMap;
use jsonc_parser::{ParseOptions, parse_to_serde_value};
use serde::Deserialize;

use super::{
    package::node_modules_bin_dirs,
    pkg_json::{PackageJson, read_package_json},
    types::{NativeDenoTaskExecution, NativeDenoTaskStage, NativeDenoTaskStep},
};

#[derive(Debug, Clone)]
pub(crate) struct DenoProject {
    pub root: PathBuf,
    pub config_path: Option<PathBuf>,
    pub deno_tasks: IndexMap<String, DenoTaskDefinition>,
    pub package_json: Option<PackageJson>,
    pub has_workspace: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DenoTaskDefinition {
    pub command: Option<String>,
    pub description: Option<String>,
    pub dependencies: Vec<String>,
}

pub(crate) fn find_nearest_deno_project(cwd: &Path) -> Result<Option<DenoProject>> {
    for dir in cwd.ancestors() {
        let deno = read_deno_config(dir)?;
        let package_json = read_package_json(dir)?;
        if deno.is_none() && package_json.is_none() {
            continue;
        }

        let (config_path, deno_tasks, has_workspace) = deno
            .map(|config| (Some(config.path), config.tasks, config.has_workspace))
            .unwrap_or_else(|| (None, IndexMap::new(), false));

        return Ok(Some(DenoProject {
            root: dir.to_path_buf(),
            config_path,
            deno_tasks,
            package_json,
            has_workspace,
        }));
    }

    Ok(None)
}

pub(crate) fn plan_native_deno_task(
    project: &DenoProject,
    selection: &str,
    forwarded_args: &[String],
    has_if_present: bool,
) -> Result<NativeDenoTaskExecution, String> {
    if project.has_workspace {
        return Err("deno workspace task execution stays in package-manager mode".to_string());
    }

    let deno_matches = match_deno_tasks(&project.deno_tasks, selection);
    if !deno_matches.is_empty() {
        let stages = build_deno_task_stages(&project.deno_tasks, &deno_matches, forwarded_args)?;
        return Ok(NativeDenoTaskExecution {
            project_root: project.root.clone(),
            config_path: project.config_path.clone(),
            selection: selection.to_string(),
            stages,
            forwarded_args: forwarded_args.to_vec(),
            bin_paths: node_modules_bin_dirs(&project.root),
        });
    }

    if let Some(package_json) = &project.package_json
        && let Some(stages) = build_package_json_stages(package_json, selection, forwarded_args)
    {
        return Ok(NativeDenoTaskExecution {
            project_root: project.root.clone(),
            config_path: project.config_path.clone(),
            selection: selection.to_string(),
            stages,
            forwarded_args: forwarded_args.to_vec(),
            bin_paths: node_modules_bin_dirs(&project.root),
        });
    }

    if has_if_present
        && (project.config_path.is_some()
            || project
                .package_json
                .as_ref()
                .and_then(|package_json| package_json.scripts.as_ref())
                .is_some())
    {
        return Ok(NativeDenoTaskExecution {
            project_root: project.root.clone(),
            config_path: project.config_path.clone(),
            selection: selection.to_string(),
            stages: Vec::new(),
            forwarded_args: forwarded_args.to_vec(),
            bin_paths: node_modules_bin_dirs(&project.root),
        });
    }

    Err(format!(
        "task '{selection}' was not found in the nearest deno project"
    ))
}

fn build_package_json_stages(
    package_json: &PackageJson,
    selection: &str,
    forwarded_args: &[String],
) -> Option<Vec<NativeDenoTaskStage>> {
    let scripts = package_json.scripts.as_ref()?;
    let main = scripts.get(selection)?;
    let mut stages = Vec::new();

    if let Some(pre) = scripts.get(&format!("pre{selection}")) {
        stages.push(NativeDenoTaskStage {
            steps: vec![NativeDenoTaskStep {
                task_name: format!("pre{selection}"),
                command: pre.clone(),
                forward_args: false,
            }],
        });
    }

    stages.push(NativeDenoTaskStage {
        steps: vec![NativeDenoTaskStep {
            task_name: selection.to_string(),
            command: main.clone(),
            forward_args: !forwarded_args.is_empty(),
        }],
    });

    if let Some(post) = scripts.get(&format!("post{selection}")) {
        stages.push(NativeDenoTaskStage {
            steps: vec![NativeDenoTaskStep {
                task_name: format!("post{selection}"),
                command: post.clone(),
                forward_args: false,
            }],
        });
    }

    Some(stages)
}

fn build_deno_task_stages(
    tasks: &IndexMap<String, DenoTaskDefinition>,
    roots: &[String],
    forwarded_args: &[String],
) -> Result<Vec<NativeDenoTaskStage>, String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum VisitState {
        Visiting,
        Visited,
    }

    let positions = tasks
        .keys()
        .enumerate()
        .map(|(idx, name)| (name.clone(), idx))
        .collect::<HashMap<_, _>>();

    let mut visit_states = HashMap::<String, VisitState>::new();
    let mut selected = HashSet::<String>::new();
    let mut indegree = HashMap::<String, usize>::new();
    let mut outgoing = HashMap::<String, Vec<String>>::new();

    fn resolve_pattern(
        tasks: &IndexMap<String, DenoTaskDefinition>,
        pattern: &str,
    ) -> Result<Vec<String>, String> {
        if pattern.contains('*') {
            let matches = match_deno_tasks(tasks, pattern);
            if matches.is_empty() {
                return Err(format!("task pattern '{pattern}' matched no deno tasks"));
            }
            return Ok(matches);
        }

        if !tasks.contains_key(pattern) {
            return Err(format!("task '{pattern}' was not found in deno.json"));
        }

        Ok(vec![pattern.to_string()])
    }

    fn visit_task(
        task_name: &str,
        tasks: &IndexMap<String, DenoTaskDefinition>,
        visit_states: &mut HashMap<String, VisitState>,
        selected: &mut HashSet<String>,
        indegree: &mut HashMap<String, usize>,
        outgoing: &mut HashMap<String, Vec<String>>,
        stack: &mut Vec<String>,
    ) -> Result<(), String> {
        if let Some(state) = visit_states.get(task_name) {
            return match state {
                VisitState::Visited => Ok(()),
                VisitState::Visiting => {
                    let mut cycle = stack.clone();
                    cycle.push(task_name.to_string());
                    Err(format!("task cycle detected: {}", cycle.join(" -> ")))
                }
            };
        }

        let task = tasks
            .get(task_name)
            .ok_or_else(|| format!("task '{task_name}' was not found in deno.json"))?;
        visit_states.insert(task_name.to_string(), VisitState::Visiting);
        selected.insert(task_name.to_string());
        indegree.entry(task_name.to_string()).or_insert(0);
        stack.push(task_name.to_string());

        for dependency in &task.dependencies {
            for matched in resolve_pattern(tasks, dependency)? {
                visit_task(
                    &matched,
                    tasks,
                    visit_states,
                    selected,
                    indegree,
                    outgoing,
                    stack,
                )?;
                let entry = outgoing.entry(matched.clone()).or_default();
                if !entry.contains(&task_name.to_string()) {
                    entry.push(task_name.to_string());
                    *indegree.entry(task_name.to_string()).or_insert(0) += 1;
                }
            }
        }

        stack.pop();
        visit_states.insert(task_name.to_string(), VisitState::Visited);
        Ok(())
    }

    for root in roots {
        for matched in resolve_pattern(tasks, root)? {
            visit_task(
                &matched,
                tasks,
                &mut visit_states,
                &mut selected,
                &mut indegree,
                &mut outgoing,
                &mut Vec::new(),
            )?;
        }
    }

    let mut completed = HashSet::<String>::new();
    let mut stages = Vec::new();
    let root_set = roots.iter().cloned().collect::<HashSet<_>>();
    let last_root = roots.last().cloned();

    while completed.len() < selected.len() {
        let mut ready = selected
            .iter()
            .filter(|name| !completed.contains(*name))
            .filter(|name| indegree.get(*name).copied().unwrap_or_default() == 0)
            .cloned()
            .collect::<Vec<_>>();
        ready.sort_by_key(|name| positions.get(name).copied().unwrap_or(usize::MAX));

        if ready.is_empty() {
            return Err("task cycle detected".to_string());
        }

        let mut steps = Vec::new();
        for name in &ready {
            if let Some(task) = tasks.get(name)
                && let Some(command) = &task.command
            {
                let forward_args = !forwarded_args.is_empty()
                    && root_set.contains(name)
                    && last_root.as_ref() == Some(name);
                steps.push(NativeDenoTaskStep {
                    task_name: name.clone(),
                    command: command.clone(),
                    forward_args,
                });
            }
        }

        if !steps.is_empty() {
            stages.push(NativeDenoTaskStage { steps });
        }

        for name in ready {
            completed.insert(name.clone());
            if let Some(children) = outgoing.get(&name) {
                for child in children {
                    if let Some(value) = indegree.get_mut(child) {
                        *value -= 1;
                    }
                }
            }
        }
    }

    Ok(stages)
}

fn match_deno_tasks(tasks: &IndexMap<String, DenoTaskDefinition>, pattern: &str) -> Vec<String> {
    if !pattern.contains('*') {
        return if tasks.contains_key(pattern) {
            vec![pattern.to_string()]
        } else {
            Vec::new()
        };
    }

    tasks
        .keys()
        .filter(|name| wildcard_matches(pattern, name))
        .cloned()
        .collect()
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let parts = pattern.split('*').collect::<Vec<_>>();
    if parts.len() == 1 {
        return pattern == value;
    }

    let mut remaining = value;
    let mut is_first = true;
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if is_first && !pattern.starts_with('*') {
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
            is_first = false;
            continue;
        }

        if index == parts.len() - 1 && !pattern.ends_with('*') {
            return remaining.ends_with(part);
        }

        let Some(position) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[position + part.len()..];
        is_first = false;
    }

    pattern.ends_with('*') || remaining.is_empty()
}

#[derive(Debug, Clone)]
struct ParsedDenoConfig {
    path: PathBuf,
    tasks: IndexMap<String, DenoTaskDefinition>,
    has_workspace: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct RawDenoConfig {
    tasks: IndexMap<String, RawDenoTask>,
    workspace: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum RawDenoTask {
    Command(String),
    Detail {
        command: Option<String>,
        description: Option<String>,
        #[serde(default)]
        dependencies: Vec<String>,
    },
}

fn read_deno_config(dir: &Path) -> Result<Option<ParsedDenoConfig>> {
    let path = if dir.join("deno.json").is_file() {
        dir.join("deno.json")
    } else if dir.join("deno.jsonc").is_file() {
        dir.join("deno.jsonc")
    } else {
        return Ok(None);
    };

    let raw = fs::read_to_string(&path)
        .map_err(|error| anyhow!("config error: failed to read {}: {error}", path.display()))?;
    let config = parse_to_serde_value::<RawDenoConfig>(&raw, &ParseOptions::default())
        .map_err(|error| anyhow!("config error: failed to parse {}: {error}", path.display()))?;
    let tasks = config
        .tasks
        .into_iter()
        .map(|(name, task)| {
            let normalized = match task {
                RawDenoTask::Command(command) => DenoTaskDefinition {
                    command: Some(command),
                    description: None,
                    dependencies: Vec::new(),
                },
                RawDenoTask::Detail {
                    command,
                    description,
                    dependencies,
                } => DenoTaskDefinition {
                    command,
                    description,
                    dependencies,
                },
            };
            (name, normalized)
        })
        .collect();

    Ok(Some(ParsedDenoConfig {
        path,
        tasks,
        has_workspace: config.workspace.is_some(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn wildcard_matches_expected_patterns() {
        assert!(wildcard_matches("build-*", "build-a"));
        assert!(wildcard_matches("build-*", "build-a-dev"));
        assert!(wildcard_matches("*-dev", "build-dev"));
        assert!(wildcard_matches("build-*-dev", "build-a-dev"));
        assert!(!wildcard_matches("build-*", "test-a"));
        assert!(!wildcard_matches("build-*-dev", "build-dev"));
    }

    #[test]
    fn reads_jsonc_tasks_and_preserves_order() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("deno.jsonc"),
            r#"{
              // comment
              "tasks": {
                "b": "echo b",
                "a": {
                  "command": "echo a",
                  "description": "task a",
                  "dependencies": ["b"]
                }
              }
            }"#,
        )
        .unwrap();

        let config = read_deno_config(dir.path()).unwrap().unwrap();
        assert_eq!(
            config.tasks.keys().cloned().collect::<Vec<_>>(),
            vec!["b", "a"]
        );
        assert_eq!(config.tasks["a"].description.as_deref(), Some("task a"));
    }

    #[test]
    fn selects_nearest_project_with_deno_or_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let nested = root.join("app").join("src");
        fs::create_dir_all(&nested).unwrap();
        fs::write(root.join("deno.json"), r#"{"tasks":{"dev":"echo ok"}}"#).unwrap();

        let project = find_nearest_deno_project(&nested).unwrap().unwrap();
        assert_eq!(project.root, root);
    }
}
