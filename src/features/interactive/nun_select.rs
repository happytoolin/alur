use std::{collections::BTreeSet, path::Path};

use anyhow::{Context, Result, anyhow};
use dialoguer::{MultiSelect, theme::ColorfulTheme};

use crate::core::pkg_json::{PackageJson, read_package_json};

pub fn choose_dependencies_for_uninstall(cwd: &Path) -> Result<Vec<String>> {
    let Some(pkg) = read_package_json(cwd)? else {
        return Err(anyhow!("interactive error: package.json not found"));
    };

    let items = dependency_items(&pkg);
    if items.is_empty() {
        return Err(anyhow!(
            "interactive error: no dependencies found in package.json"
        ));
    }

    let selected = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select dependencies to remove")
        .items(&items)
        .interact()
        .context("interactive error: failed to read dependency selection")?;

    if selected.is_empty() {
        return Ok(Vec::new());
    }

    Ok(selected.into_iter().map(|i| items[i].clone()).collect())
}

fn dependency_items(pkg: &PackageJson) -> Vec<String> {
    let mut deps = BTreeSet::new();
    for map in [
        &pkg.dependencies,
        &pkg.dev_dependencies,
        &pkg.peer_dependencies,
        &pkg.optional_dependencies,
    ]
    .into_iter()
    .flatten()
    {
        deps.extend(map.keys().cloned());
    }
    deps.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn merges_dependencies_from_all_supported_sections() {
        let pkg = PackageJson {
            dependencies: Some(BTreeMap::from([("react".to_string(), "18".to_string())])),
            dev_dependencies: Some(BTreeMap::from([("vitest".to_string(), "1".to_string())])),
            peer_dependencies: Some(BTreeMap::from([(
                "@types/react".to_string(),
                "18".to_string(),
            )])),
            optional_dependencies: Some(BTreeMap::from([(
                "fsevents".to_string(),
                "2".to_string(),
            )])),
            ..PackageJson::default()
        };

        let out = dependency_items(&pkg);
        assert_eq!(out, vec!["@types/react", "fsevents", "react", "vitest"]);
    }

    #[test]
    fn deduplicates_duplicate_dependency_names_across_sections() {
        let pkg = PackageJson {
            dependencies: Some(BTreeMap::from([("react".to_string(), "18".to_string())])),
            dev_dependencies: Some(BTreeMap::from([("react".to_string(), "18".to_string())])),
            ..PackageJson::default()
        };

        let out = dependency_items(&pkg);
        assert_eq!(out, vec!["react"]);
    }

    #[test]
    fn returns_empty_when_no_dependencies_exist() {
        let out = dependency_items(&PackageJson::default());
        assert!(out.is_empty());
    }
}
