use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow};
use dialoguer::{FuzzySelect, theme::ColorfulTheme};

use crate::core::resolve::ResolveContext;

pub fn read_scripts(ctx: &ResolveContext) -> Result<Vec<ScriptEntry>> {
    let Some(pkg) = ctx.project_state()?.nearest_package() else {
        return Ok(Vec::new());
    };

    let scripts = pkg.manifest.scripts.unwrap_or_default();
    let scripts_info = pkg.manifest.scripts_info.unwrap_or_default();

    Ok(build_script_entries(&scripts, &scripts_info))
}

pub fn choose_script_interactive(ctx: &ResolveContext) -> Result<String> {
    let scripts = read_scripts(ctx)?;
    if scripts.is_empty() {
        return Err(anyhow!(
            "interactive error: no scripts found in package.json"
        ));
    }

    let labels: Vec<String> = scripts
        .iter()
        .map(|entry| format!("{}: {}", entry.name, entry.description))
        .collect();

    let idx = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose script to run")
        .items(&labels)
        .default(0)
        .interact_opt()
        .context("interactive error: failed to read script selection")?
        .ok_or_else(|| anyhow!("interactive error: script selection canceled"))?;

    Ok(scripts[idx].name.clone())
}

#[derive(Debug, Clone)]
pub struct ScriptEntry {
    pub name: String,
    pub description: String,
}

fn build_script_entries(
    scripts: &BTreeMap<String, String>,
    scripts_info: &BTreeMap<String, String>,
) -> Vec<ScriptEntry> {
    scripts
        .iter()
        .filter(|(name, _)| !name.starts_with('?'))
        .map(|(name, cmd)| {
            let description = scripts_info
                .get(name)
                .cloned()
                .or_else(|| scripts.get(&format!("?{name}")).cloned())
                .unwrap_or_else(|| cmd.clone());

            ScriptEntry {
                name: name.clone(),
                description,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_hidden_question_mark_scripts() {
        let scripts = BTreeMap::from([
            ("dev".to_string(), "vite".to_string()),
            ("?dev".to_string(), "hidden".to_string()),
        ]);
        let out = build_script_entries(&scripts, &BTreeMap::new());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "dev");
    }

    #[test]
    fn prefers_scripts_info_description() {
        let scripts = BTreeMap::from([("dev".to_string(), "vite".to_string())]);
        let scripts_info = BTreeMap::from([("dev".to_string(), "Start dev server".to_string())]);
        let out = build_script_entries(&scripts, &scripts_info);
        assert_eq!(out[0].description, "Start dev server");
    }

    #[test]
    fn falls_back_to_question_mark_script_description() {
        let scripts = BTreeMap::from([
            ("dev".to_string(), "vite".to_string()),
            ("?dev".to_string(), "Run local dev server".to_string()),
        ]);
        let out = build_script_entries(&scripts, &BTreeMap::new());
        assert_eq!(out[0].description, "Run local dev server");
    }

    #[test]
    fn falls_back_to_script_command_when_no_description_available() {
        let scripts = BTreeMap::from([("build".to_string(), "vite build".to_string())]);
        let out = build_script_entries(&scripts, &BTreeMap::new());
        assert_eq!(out[0].description, "vite build");
    }

    #[test]
    fn entries_are_stably_sorted_by_script_name() {
        let scripts = BTreeMap::from([
            ("test".to_string(), "vitest".to_string()),
            ("build".to_string(), "vite build".to_string()),
        ]);
        let out = build_script_entries(&scripts, &BTreeMap::new());
        assert_eq!(out[0].name, "build");
        assert_eq!(out[1].name, "test");
    }
}
