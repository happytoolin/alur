use anyhow::Result;

use crate::{
    core::{
        resolve::{self, ResolveContext},
        types::ResolvedExecution,
    },
    features::interactive::nr_scripts::choose_script_interactive,
};

pub fn handle(mut args: Vec<String>, ctx: &ResolveContext) -> Result<Option<ResolvedExecution>> {
    if args.is_empty() {
        args.push(choose_script_interactive(ctx)?);
    }

    let resolved = resolve::resolve_nr(args, ctx)?;
    Ok(Some(resolved))
}
