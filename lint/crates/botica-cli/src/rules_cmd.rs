use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::project;

#[derive(Args)]
pub struct RulesArgs {
    /// Also list the rules that are not active, with the reason.
    #[arg(long)]
    all: bool,
    /// Use this configuration file instead of looking for one.
    #[arg(long)]
    config: Option<PathBuf>,
}

pub fn run(args: &RulesArgs) -> Result<()> {
    let project = project::load(args.config.as_deref())?;
    let (loaded, resolution) = (&project.loaded, &project.resolution);

    let mut out = std::io::stdout().lock();
    match &loaded.source {
        Some(path) => writeln!(out, "config: {}", path.display())?,
        None => writeln!(
            out,
            "config: none (the defaults apply; root {})",
            loaded.root.display()
        )?,
    }
    for entry in resolution
        .entries()
        .iter()
        .filter(|entry| args.all || entry.active)
    {
        let state = if entry.active { "active" } else { "inactive" };
        writeln!(
            out,
            "{:<12} {:<22} {:<9} {}",
            entry.rule.code, entry.rule.id, state, entry.reason
        )?;
    }
    Ok(())
}
