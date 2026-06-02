use anyhow::{Context, Result, anyhow};
use dialoguer::{FuzzySelect, Input, Select, theme::ColorfulTheme};
use serde::Deserialize;
use std::time::Duration;

use crate::core::{resolve::ResolveContext, types::PackageManager};

#[derive(Debug, Deserialize)]
struct NpmResponse {
    objects: Vec<NpmEntry>,
}

#[derive(Debug, Deserialize)]
struct NpmEntry {
    package: NpmPackage,
}

#[derive(Debug, Deserialize)]
struct NpmPackage {
    name: String,
    version: String,
    description: Option<String>,
}

pub fn augment_ni_args_interactive(args: Vec<String>, ctx: &ResolveContext) -> Result<Vec<String>> {
    let is_interactive = interactive_requested(&args);
    if !is_interactive {
        return Ok(args);
    }

    let args = strip_interactive_flags(args);

    let pattern = args
        .iter()
        .find(|v| !v.starts_with('-'))
        .cloned()
        .map_or_else(prompt_pattern, Ok)?;

    if pattern.trim().is_empty() {
        return Err(anyhow!(
            "interactive error: interactive install requires a package search pattern"
        ));
    }

    let packages = fetch_npm_packages(&pattern)?;
    if packages.is_empty() {
        return Err(anyhow!(
            "interactive error: no npm packages found for pattern '{pattern}'"
        ));
    }

    let labels: Vec<String> = packages
        .iter()
        .map(|pkg| {
            let description = pkg.description.as_deref().unwrap_or("");
            format!("{}  v{}  {}", pkg.name, pkg.version, description)
        })
        .collect();

    let idx = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose package to install")
        .items(&labels)
        .default(0)
        .interact_opt()
        .context("interactive error: failed to read package selection")?
        .ok_or_else(|| anyhow!("interactive error: package selection canceled"))?;

    let chosen = &packages[idx].name;

    let agent = crate::core::resolve::detected_package_manager(ctx)?;
    let mode_labels = mode_labels_for(agent);
    let mode_idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(format!("Install {chosen} as"))
        .items(&mode_labels)
        .default(0)
        .interact()
        .context("interactive error: failed to read install mode")?;

    Ok(apply_selected_package(args, chosen, mode_labels[mode_idx]))
}

fn interactive_requested(args: &[String]) -> bool {
    args.iter()
        .any(|arg| matches!(arg.as_str(), "-i" | "--interactive"))
}

fn strip_interactive_flags(args: Vec<String>) -> Vec<String> {
    args.into_iter()
        .filter(|arg| !matches!(arg.as_str(), "-i" | "--interactive"))
        .collect()
}

fn mode_labels_for(agent: PackageManager) -> Vec<&'static str> {
    if matches!(agent, PackageManager::Npm | PackageManager::Pnpm) {
        vec!["prod", "dev", "peer"]
    } else {
        vec!["prod", "dev"]
    }
}

fn apply_selected_package(mut args: Vec<String>, chosen: &str, mode: &str) -> Vec<String> {
    let mut removed_search_query = false;
    args.retain(|arg| {
        if !removed_search_query && !arg.starts_with('-') {
            removed_search_query = true;
            return false;
        }
        !matches!(
            arg.as_str(),
            "-d" | "-D" | "-p" | "--dev" | "--peer" | "--save-dev" | "--save-peer"
        )
    });
    args.push(chosen.to_string());

    match mode {
        "dev" => args.push("-D".to_string()),
        "peer" => args.push("--save-peer".to_string()),
        _ => {}
    }

    args
}

fn prompt_pattern() -> Result<String> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt("search for package")
        .interact_text()
        .context("interactive error: failed to read interactive search pattern")
}

fn fetch_npm_packages(pattern: &str) -> Result<Vec<NpmPackage>> {
    let mut response = ureq::get(&npm_search_url(pattern))
        .config()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .call()
        .context("network error: failed to query npm registry")?;

    let parsed = response
        .body_mut()
        .read_json::<NpmResponse>()
        .context("network error: failed to parse npm registry response")?;

    Ok(parsed.objects.into_iter().map(|obj| obj.package).collect())
}

fn npm_search_url(pattern: &str) -> String {
    format!(
        "https://registry.npmjs.com/-/v1/search?text={}&size=35",
        urlencoding::encode(pattern)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{config::HniConfig, resolve::ResolveContext};
    use std::path::PathBuf;

    #[test]
    fn detects_interactive_request_from_short_or_long_flag() {
        assert!(interactive_requested(&["-i".to_string()]));
        assert!(interactive_requested(&["--interactive".to_string()]));
        assert!(!interactive_requested(&["react".to_string()]));
    }

    #[test]
    fn strips_interactive_flags_only() {
        let out = strip_interactive_flags(vec![
            "react".to_string(),
            "-i".to_string(),
            "--interactive".to_string(),
            "-D".to_string(),
        ]);
        assert_eq!(out, vec!["react", "-D"]);
    }

    #[test]
    fn mode_labels_include_peer_only_for_npm_and_pnpm() {
        assert_eq!(
            mode_labels_for(PackageManager::Npm),
            vec!["prod", "dev", "peer"]
        );
        assert_eq!(
            mode_labels_for(PackageManager::Pnpm),
            vec!["prod", "dev", "peer"]
        );
        assert_eq!(mode_labels_for(PackageManager::Yarn), vec!["prod", "dev"]);
        assert_eq!(
            mode_labels_for(PackageManager::YarnBerry),
            vec!["prod", "dev"]
        );
        assert_eq!(mode_labels_for(PackageManager::Bun), vec!["prod", "dev"]);
        assert_eq!(mode_labels_for(PackageManager::Deno), vec!["prod", "dev"]);
    }

    #[test]
    fn apply_selected_package_replaces_search_query_and_dev_mode_flags() {
        let out = apply_selected_package(
            vec![
                "--frozen".to_string(),
                "react".to_string(),
                "--save-peer".to_string(),
                "-d".to_string(),
                "--peer".to_string(),
                "@types/react".to_string(),
            ],
            "vue",
            "prod",
        );
        assert_eq!(out, vec!["--frozen", "@types/react", "vue"]);
    }

    #[test]
    fn apply_selected_package_adds_dev_flag() {
        let out = apply_selected_package(vec!["react".to_string()], "vue", "dev");
        assert_eq!(out, vec!["vue", "-D"]);
    }

    #[test]
    fn apply_selected_package_adds_peer_flag() {
        let out = apply_selected_package(vec!["react".to_string()], "vue", "peer");
        assert_eq!(out, vec!["vue", "--save-peer"]);
    }

    #[test]
    fn non_interactive_call_is_passthrough() {
        let ctx = ResolveContext::new(PathBuf::from("."), HniConfig::default());
        let input = vec!["react".to_string(), "-D".to_string()];
        let out = augment_ni_args_interactive(input.clone(), &ctx).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn npm_search_url_encodes_pattern() {
        let url = npm_search_url("react+dom @types");
        assert_eq!(
            url,
            "https://registry.npmjs.com/-/v1/search?text=react%2Bdom%20%40types&size=35"
        );
    }
}
