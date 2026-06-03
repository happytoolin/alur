use anyhow::{Result, anyhow};

use crate::core::{
    batch,
    resolve::{self, ResolveContext},
    types::{BatchMode, ResolvedExecution},
};

pub fn handle_ni(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(resolve::resolve_ni(args, ctx)?))
}

pub fn handle_nr(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(resolve::resolve_nr(args, ctx)?))
}

pub fn handle_nlx(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    if args.is_empty() {
        return Err(anyhow!(
            "execution error: nlx requires a command to execute.\nTry: nlx create-vite@latest"
        ));
    }

    Ok(Some(resolve::resolve_nlx(args, ctx)?))
}

pub fn handle_nun(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(resolve::resolve_nun(args, ctx)?))
}

pub fn handle_nci(args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    Ok(Some(resolve::resolve_nci(args, ctx)?))
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
