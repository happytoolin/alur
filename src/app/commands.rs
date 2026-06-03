use anyhow::{Result, anyhow};

use crate::{
    core::{
        batch,
        resolve::{self, ResolveContext},
        types::{BatchMode, ResolvedExecution},
    },
    features::{
        interactive::{
            ni_search::augment_ni_args_interactive, nun_select::choose_dependencies_for_uninstall,
        },
        nr,
    },
};

use super::completion::{nr_completion_script_for, print_nr_completion_query};

pub fn handle_ni(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    let args = augment_ni_args_interactive(args, ctx)?;
    Ok(Some(resolve::resolve_ni(args, ctx)?))
}

pub fn handle_nr(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    if let Some(script) = args
        .first()
        .and_then(|first| nr_completion_script_for(first.as_str()))
    {
        println!("{script}");
        return Ok(None);
    }

    if args.first().is_some_and(|first| first == "--completion") {
        print_nr_completion_query(&args[1..], ctx)?;
        return Ok(None);
    }

    nr::handle(args, ctx)
}

pub fn handle_nlx(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    if args.is_empty() {
        return Err(anyhow!(
            "execution error: nlx requires a command to execute.\nTry: nlx create-vite@latest"
        ));
    }

    Ok(Some(resolve::resolve_nlx(args, ctx)?))
}

pub fn handle_nru(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(resolve::resolve_nru(args, ctx)?))
}

pub fn handle_nun(
    mut args: Vec<String>,
    ctx: &ResolveContext,
) -> Result<Option<ResolvedExecution>> {
    let interactive_multi = args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-m" | "--multi-select"));

    args.retain(|arg| !matches!(arg.as_str(), "-m" | "--multi-select"));

    if args.is_empty() || interactive_multi {
        let selected = choose_dependencies_for_uninstall(ctx.cwd())?;
        if selected.is_empty() {
            return Ok(None);
        }

        args.extend(selected);
    }

    Ok(Some(resolve::resolve_nun(args, ctx)?))
}

pub fn handle_nci(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(resolve::resolve_nci(args, ctx)?))
}

pub fn handle_na(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    if args.is_empty() {
        println!("{}", resolve::detected_package_manager(ctx)?.display_name());
        return Ok(None);
    }

    Ok(Some(resolve::resolve_na(args, ctx)?))
}

pub fn handle_np(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(batch::make_execution(
        BatchMode::Parallel,
        args,
        ctx.cwd(),
    )))
}

pub fn handle_ns(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(batch::make_execution(
        BatchMode::Sequential,
        args,
        ctx.cwd(),
    )))
}
