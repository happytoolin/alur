use std::collections::BTreeSet;

pub fn completion_candidates(
    prefix: &str,
    scripts: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let prefix = prefix.trim();
    let set: BTreeSet<String> = scripts
        .into_iter()
        .filter(|script| script.starts_with(prefix))
        .collect();
    set.into_iter().collect()
}
