// Portions adapted from nubjs/nub at fd78dc6, MIT licensed.
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use wildmatch::WildMatch;

use crate::core::pkg_json::{PackageJson, read_package_json};

#[derive(Debug, Clone)]
pub(crate) struct Workspace {
    pub root: PathBuf,
    pub root_package: Option<WorkspacePackage>,
    pub members: Vec<WorkspacePackage>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkspacePackage {
    pub name: String,
    pub dir: PathBuf,
    pub manifest: PackageJson,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct WorkspaceSelectionOptions {
    pub filters: Vec<String>,
    pub workspace_root: bool,
    pub include_workspace_root: bool,
    pub fail_if_no_match: bool,
    pub parallel: bool,
    pub stream: bool,
    pub workspace_concurrency: Option<i32>,
    pub resume_from: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceSelection {
    pub chunks: Vec<Vec<WorkspacePackage>>,
}

#[derive(Debug, Clone)]
struct Filter {
    pattern: String,
    include_dependencies: bool,
    include_dependents: bool,
    exclude_self: bool,
    exclude: bool,
}

impl Filter {
    fn parse(value: &str) -> Self {
        let mut pattern = value.to_string();
        let mut include_dependencies = false;
        let mut include_dependents = false;
        let mut exclude_self = false;
        let mut exclude = false;

        if let Some(rest) = pattern.strip_prefix('!') {
            exclude = true;
            pattern = rest.to_string();
        }

        if pattern.ends_with("...") {
            include_dependencies = true;
            pattern.truncate(pattern.len() - 3);
            if pattern.ends_with('^') {
                exclude_self = true;
                pattern.pop();
            }
        }

        if let Some(rest) = pattern.strip_prefix("...") {
            include_dependents = true;
            pattern = rest.to_string();
            if let Some(rest) = pattern.strip_prefix('^') {
                exclude_self = true;
                pattern = rest.to_string();
            }
        }

        if pattern.starts_with('{') && pattern.ends_with('}') {
            let inner = &pattern[1..pattern.len() - 1];
            pattern = if inner.starts_with('.') {
                inner.to_string()
            } else {
                format!("./{inner}")
            };
        }

        Self {
            pattern,
            include_dependencies,
            include_dependents,
            exclude_self,
            exclude,
        }
    }
}

pub(crate) fn select_workspace_packages(
    cwd: &Path,
    opts: &WorkspaceSelectionOptions,
) -> Result<WorkspaceSelection> {
    let workspace = discover_workspace(cwd)?
        .ok_or_else(|| anyhow!("workspace fast mode requires a workspace root"))?;

    let selected = if opts.workspace_root {
        workspace.root_package.clone().into_iter().collect()
    } else {
        let filters = opts
            .filters
            .iter()
            .map(|filter| Filter::parse(filter))
            .collect::<Vec<_>>();
        let mut indices = apply_filters(&workspace.members, &filters, &workspace.root);

        if let Some(resume_from) = &opts.resume_from
            && let Some(position) = indices
                .iter()
                .position(|&idx| workspace.members[idx].name == *resume_from)
        {
            indices.drain(..position);
        }

        let mut selected = indices
            .into_iter()
            .map(|idx| workspace.members[idx].clone())
            .collect::<Vec<_>>();
        if opts.include_workspace_root
            && let Some(root) = workspace.root_package.clone()
            && selected.iter().all(|pkg| pkg.dir != root.dir)
        {
            selected.insert(0, root);
        }
        selected
    };

    if opts.fail_if_no_match && selected.is_empty() {
        return Err(anyhow!("workspace filter matched no packages"));
    }

    let chunks = package_chunks(&selected);
    Ok(WorkspaceSelection { chunks })
}

pub(crate) fn discover_workspace(cwd: &Path) -> Result<Option<Workspace>> {
    for dir in cwd.ancestors() {
        let Some(patterns) = workspace_patterns(dir)? else {
            continue;
        };
        let root_package = read_workspace_package(dir)?;
        let members = expand_member_patterns(dir, &patterns)?;
        return Ok(Some(Workspace {
            root: dir.to_path_buf(),
            root_package,
            members,
        }));
    }
    Ok(None)
}

fn workspace_patterns(dir: &Path) -> Result<Option<Vec<String>>> {
    let pkg_path = dir.join("package.json");
    if let Ok(raw) = fs::read(&pkg_path) {
        let manifest = serde_json::from_slice::<serde_json::Value>(&raw)
            .map_err(|err| anyhow!("failed to parse {}: {err}", pkg_path.display()))?;
        if let Some(patterns) = workspace_patterns_from_manifest(&manifest) {
            return Ok(Some(patterns));
        }
    }

    if dir.join("pnpm-lock.yaml").is_file()
        && let Some(patterns) = read_pnpm_workspace(dir)
    {
        return Ok(Some(patterns));
    }

    Ok(None)
}

fn workspace_patterns_from_manifest(manifest: &serde_json::Value) -> Option<Vec<String>> {
    match manifest.get("workspaces") {
        Some(serde_json::Value::Array(values)) => Some(string_array_values(values)),
        Some(serde_json::Value::Object(object)) => object
            .get("packages")
            .and_then(serde_json::Value::as_array)
            .map(|values| string_array_values(values)),
        _ => None,
    }
}

fn string_array_values(values: &[serde_json::Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| value.as_str().map(ToString::to_string))
        .collect()
}

fn read_pnpm_workspace(dir: &Path) -> Option<Vec<String>> {
    let raw = fs::read_to_string(dir.join("pnpm-workspace.yaml")).ok()?;
    let mut patterns = Vec::new();
    let mut in_packages = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "packages:" {
            in_packages = true;
            continue;
        }
        if in_packages {
            if let Some(rest) = trimmed.strip_prefix("- ") {
                patterns.push(rest.trim().trim_matches(['"', '\'']).to_string());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }
        }
    }
    (!patterns.is_empty()).then_some(patterns)
}

const MAX_GLOB_DEPTH: usize = 24;

fn expand_member_patterns(root: &Path, raw_patterns: &[String]) -> Result<Vec<WorkspacePackage>> {
    let mut include = Vec::new();
    let mut exclude = Vec::new();
    let mut max_depth = 1usize;

    for raw in raw_patterns {
        let (negated, pattern) = raw
            .strip_prefix('!')
            .map_or((false, raw.as_str()), |rest| (true, rest));
        let pattern = normalize_dir_pattern(pattern);
        if pattern.is_empty() {
            continue;
        }
        if !negated {
            max_depth = max_depth.max(pattern_depth(&pattern));
            include.push(pattern);
        } else {
            exclude.push(pattern);
        }
    }

    if include.is_empty() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::new();
    collect_package_dirs(root, PathBuf::new(), 0, max_depth, &mut candidates);

    let mut members = Vec::new();
    for relative in candidates {
        let rel = relative.to_string_lossy().replace('\\', "/");
        if !include.iter().any(|pattern| glob_matches(pattern, &rel)) {
            continue;
        }
        if exclude.iter().any(|pattern| glob_matches(pattern, &rel)) {
            continue;
        }
        let dir = root.join(&relative);
        if let Some(pkg) = read_workspace_package(&dir)? {
            members.push(pkg);
        }
    }
    Ok(members)
}

fn normalize_dir_pattern(pattern: &str) -> String {
    pattern
        .trim_start_matches("./")
        .trim_start_matches('/')
        .trim_end_matches('/')
        .replace('\\', "/")
}

fn pattern_depth(pattern: &str) -> usize {
    if pattern.contains("**") {
        MAX_GLOB_DEPTH
    } else {
        pattern.split('/').count()
    }
}

fn collect_package_dirs(
    root: &Path,
    relative: PathBuf,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<PathBuf>,
) {
    if depth > max_depth {
        return;
    }

    let Ok(entries) = fs::read_dir(root.join(&relative)) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "node_modules" || name.starts_with('.') {
            continue;
        }

        let child = relative.join(name.as_ref());
        if entry.path().join("package.json").is_file() {
            out.push(child.clone());
        }
        collect_package_dirs(root, child, depth + 1, max_depth, out);
    }
}

fn read_workspace_package(dir: &Path) -> Result<Option<WorkspacePackage>> {
    let Some(manifest) = read_package_json(dir)? else {
        return Ok(None);
    };
    let name = manifest.name.clone().unwrap_or_default();
    Ok(Some(WorkspacePackage {
        name,
        dir: dir.to_path_buf(),
        manifest,
    }))
}

fn apply_filters(
    members: &[WorkspacePackage],
    filters: &[Filter],
    workspace_root: &Path,
) -> Vec<usize> {
    let name_to_idx = members
        .iter()
        .enumerate()
        .map(|(idx, package)| (package.name.as_str(), idx))
        .collect::<HashMap<_, _>>();
    let has_includes = filters.iter().any(|filter| !filter.exclude);

    let mut selected = if has_includes {
        let mut set = HashSet::new();
        for filter in filters.iter().filter(|filter| !filter.exclude) {
            set.extend(raw_matched_set(
                members,
                filter,
                &name_to_idx,
                workspace_root,
            ));
        }
        set
    } else {
        (0..members.len()).collect::<HashSet<_>>()
    };

    for filter in filters.iter().filter(|filter| filter.exclude) {
        for idx in raw_matched_set(members, filter, &name_to_idx, workspace_root) {
            selected.remove(&idx);
        }
    }

    topological_sort(&selected, &build_dep_graph(members, &name_to_idx))
}

fn raw_matched_set(
    members: &[WorkspacePackage],
    filter: &Filter,
    name_to_idx: &HashMap<&str, usize>,
    workspace_root: &Path,
) -> HashSet<usize> {
    let mut matched = if filter.pattern.is_empty() {
        (0..members.len()).collect()
    } else {
        initial_matches(members, filter, workspace_root)
    };
    let initial = matched.clone();

    if filter.include_dependencies {
        let deps = build_dep_graph(members, name_to_idx);
        for idx in initial.iter().copied() {
            traverse(&deps, idx, &mut matched);
        }
    }

    if filter.include_dependents {
        let deps = build_reverse_dep_graph(members, name_to_idx);
        for idx in initial.iter().copied() {
            traverse(&deps, idx, &mut matched);
        }
    }

    if filter.exclude_self {
        for idx in initial {
            matched.remove(&idx);
        }
    }

    matched
}

fn initial_matches(
    members: &[WorkspacePackage],
    filter: &Filter,
    workspace_root: &Path,
) -> HashSet<usize> {
    let mut matched = HashSet::new();
    for (idx, package) in members.iter().enumerate() {
        let rel_dir = package
            .dir
            .strip_prefix(workspace_root)
            .unwrap_or(package.dir.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        if matches_package_pattern(&package.name, &rel_dir, &filter.pattern) {
            matched.insert(idx);
        }
    }

    if matched.is_empty() && is_bare_name(&filter.pattern) {
        let unscoped = members
            .iter()
            .enumerate()
            .filter(|(_, package)| unscoped_name(&package.name) == filter.pattern)
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>();
        if unscoped.len() == 1 {
            matched.insert(unscoped[0]);
        }
    }

    matched
}

fn matches_package_pattern(name: &str, rel_dir: &str, pattern: &str) -> bool {
    if name == pattern {
        return true;
    }

    if let Some(dir_pattern) = pattern.strip_prefix("./") {
        let dir_pattern = dir_pattern.trim_end_matches('/');
        if has_glob(dir_pattern) {
            return glob_matches(dir_pattern, rel_dir);
        }
        return rel_dir == dir_pattern;
    }

    has_glob(pattern) && glob_matches(pattern, name)
}

fn has_glob(pattern: &str) -> bool {
    pattern.contains(['*', '?', '[', ']'])
}

fn glob_matches(pattern: &str, value: &str) -> bool {
    WildMatch::new(pattern).matches(value)
}

fn is_bare_name(pattern: &str) -> bool {
    !pattern.is_empty()
        && !pattern.contains(['@', '/', '*', '?', '[', ']', '{'])
        && !pattern.starts_with('.')
}

fn unscoped_name(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn build_dep_graph(
    members: &[WorkspacePackage],
    name_to_idx: &HashMap<&str, usize>,
) -> Vec<HashSet<usize>> {
    members
        .iter()
        .map(|package| {
            package
                .manifest
                .dependencies
                .keys()
                .chain(package.manifest.dev_dependencies.keys())
                .chain(package.manifest.peer_dependencies.keys())
                .filter_map(|name| name_to_idx.get(name.as_str()).copied())
                .collect()
        })
        .collect()
}

fn build_reverse_dep_graph(
    members: &[WorkspacePackage],
    name_to_idx: &HashMap<&str, usize>,
) -> Vec<HashSet<usize>> {
    let forward = build_dep_graph(members, name_to_idx);
    let mut reverse = vec![HashSet::new(); members.len()];
    for (idx, deps) in forward.iter().enumerate() {
        for dep in deps {
            reverse[*dep].insert(idx);
        }
    }
    reverse
}

fn traverse(graph: &[HashSet<usize>], start: usize, visited: &mut HashSet<usize>) {
    let mut queue = VecDeque::from([start]);
    while let Some(idx) = queue.pop_front() {
        if let Some(edges) = graph.get(idx) {
            for edge in edges {
                if visited.insert(*edge) {
                    queue.push_back(*edge);
                }
            }
        }
    }
}

fn package_chunks(packages: &[WorkspacePackage]) -> Vec<Vec<WorkspacePackage>> {
    let name_to_idx = packages
        .iter()
        .enumerate()
        .map(|(idx, package)| (package.name.as_str(), idx))
        .collect::<HashMap<_, _>>();
    let nodes = (0..packages.len()).collect::<HashSet<_>>();
    topological_chunks(&nodes, &build_dep_graph(packages, &name_to_idx))
        .into_iter()
        .map(|chunk| {
            chunk
                .into_iter()
                .map(|idx| packages[idx].clone())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn topological_sort(nodes: &HashSet<usize>, deps: &[HashSet<usize>]) -> Vec<usize> {
    topological_chunks(nodes, deps)
        .into_iter()
        .flatten()
        .collect()
}

fn topological_chunks(nodes: &HashSet<usize>, deps: &[HashSet<usize>]) -> Vec<Vec<usize>> {
    let mut indegree = HashMap::new();
    let mut dependents: HashMap<usize, Vec<usize>> = HashMap::new();

    for node in nodes {
        let count = deps
            .get(*node)
            .into_iter()
            .flatten()
            .filter(|dep| nodes.contains(dep))
            .inspect(|dep| dependents.entry(**dep).or_default().push(*node))
            .count();
        indegree.insert(*node, count);
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(node, count)| (*count == 0).then_some(*node))
        .collect::<Vec<_>>();
    let mut chunks = Vec::new();
    let mut emitted = 0usize;

    while !ready.is_empty() {
        ready.sort_unstable();
        let chunk = std::mem::take(&mut ready);
        emitted += chunk.len();
        for node in &chunk {
            for dependent in dependents.get(node).into_iter().flatten() {
                if let Some(count) = indegree.get_mut(dependent) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push(*dependent);
                    }
                }
            }
        }
        chunks.push(chunk);
    }

    if emitted < nodes.len() {
        let mut leftover = nodes
            .iter()
            .copied()
            .filter(|node| indegree.get(node).copied().unwrap_or_default() > 0)
            .collect::<Vec<_>>();
        leftover.sort_unstable();
        chunks.push(leftover);
    }

    chunks
}

pub(crate) fn resolve_workspace_concurrency(value: Option<i32>) -> usize {
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    match value {
        None => cores.clamp(1, 4),
        Some(n) if n <= 0 => cores.saturating_sub(n.unsigned_abs() as usize).max(1),
        Some(n) => n as usize,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn discovers_package_json_workspace_members() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"name":"root","workspaces":["packages/*"]}"#,
        )
        .unwrap();
        let app = dir.path().join("packages/app");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("package.json"), r#"{"name":"app"}"#).unwrap();

        let workspace = discover_workspace(&app).unwrap().unwrap();
        assert_eq!(workspace.root, dir.path());
        assert_eq!(workspace.members[0].name, "app");
    }

    #[test]
    fn ignores_pnpm_workspace_without_pnpm_lock() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), r#"{"name":"root"}"#).unwrap();
        fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - packages/*\n",
        )
        .unwrap();

        assert!(discover_workspace(dir.path()).unwrap().is_none());
    }

    #[test]
    fn filters_union_exclusions_and_ellipsis() {
        let members = vec![
            package("app", &[("lib", "workspace:*")]),
            package("lib", &[("core", "workspace:*")]),
            package("core", &[]),
            package("docs", &[]),
        ];
        let filters = [Filter::parse("lib..."), Filter::parse("!docs")];
        let selected = apply_filters(&members, &filters, Path::new("."));
        let names = selected
            .iter()
            .map(|idx| members[*idx].name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["core", "lib"]);
    }

    fn package(name: &str, deps: &[(&str, &str)]) -> WorkspacePackage {
        WorkspacePackage {
            name: name.to_string(),
            dir: PathBuf::from(name),
            manifest: PackageJson {
                name: Some(name.to_string()),
                dependencies: deps
                    .iter()
                    .map(|(name, version)| ((*name).to_string(), (*version).to_string()))
                    .collect(),
                ..PackageJson::default()
            },
        }
    }
}
