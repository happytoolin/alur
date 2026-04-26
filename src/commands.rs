use anyhow::{Result, anyhow};

use crate::{
    core::{
        batch::{self, BatchMode},
        resolve::{self, ResolveContext},
        types::ResolvedExecution,
    },
    features::{
        interactive::{
            ni_search::augment_ni_args_interactive, nun_select::choose_dependencies_for_uninstall,
        },
        node_shim, nr,
    },
};

pub fn handle_ni(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    let args = augment_ni_args_interactive(args, ctx)?;
    Ok(Some(resolve::resolve_ni(args, ctx)?))
}

pub fn handle_nr(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
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

pub fn handle_node(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    node_shim::handle(args, ctx)
}
