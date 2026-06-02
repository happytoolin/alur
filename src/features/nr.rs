use std::env;

use anyhow::Result;

use crate::{
    core::{
        resolve::{self, ResolveContext},
        types::ResolvedExecution,
    },
    features::interactive::{
        completion::completion_candidates,
        nr_scripts::{choose_script_interactive, read_scripts},
    },
};

pub fn handle(mut args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    if let Some(first) = args.first()
        && first == "--completion"
    {
        handle_completion_query(&args[1..], ctx)?;
        return Ok(None);
    }

    if args.is_empty() {
        args.push(choose_script_interactive(ctx)?);
    }

    let resolved = resolve::resolve_nr(args, ctx)?;
    Ok(Some(resolved))
}

fn handle_completion_query(args: &[String], ctx: &ResolveContext) -> Result<()> {
    let scripts = read_scripts(ctx)?;
    let script_names = scripts.into_iter().map(|s| s.name).collect::<Vec<_>>();

    let comp_word = env::var("COMP_CWORD")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);

    let prefix = if comp_word > 1 {
        args.last().cloned().unwrap_or_default()
    } else {
        args.get(1).cloned().unwrap_or_default()
    };

    for candidate in completion_candidates(&prefix, script_names) {
        println!("{candidate}");
    }

    Ok(())
}
