use std::io::Write;

use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::project::{self, Project, ProjectArgs};
use crate::rule_view::RuleView;
use crate::rules_export::{self, ExportArgs};
use crate::rules_new::{self, NewArgs};

/// Version of the JSON document of `botica rules --json`.
const SCHEMA_VERSION: u32 = 1;

#[derive(Args)]
pub struct RulesArgs {
    #[command(subcommand)]
    command: Option<RulesCommand>,
    #[command(flatten)]
    list: ListArgs,
}

#[derive(Subcommand)]
enum RulesCommand {
    /// Show which rules are active and why (what `botica rules` does by itself).
    List(ListArgs),
    /// Write the catalog as documents: the site and `catalog.json` hold every rule, and the block
    /// of `AGENTS.md` the judgment rules that the project selects.
    Export(ExportArgs),
    /// Write the folder of a new rule in the catalog of a checkout, with the next code of its
    /// family, a README with the sections of its template and empty fixtures.
    New(NewArgs),
}

#[derive(Args)]
struct ListArgs {
    /// Also list the rules that are not active, with the reason.
    #[arg(long)]
    all: bool,
    /// Print one JSON document instead of the table.
    #[arg(long)]
    json: bool,
    #[command(flatten)]
    project: ProjectArgs,
}

#[derive(Serialize)]
struct ConfigView {
    /// The file the settings come from; `null` when the defaults apply.
    path: Option<String>,
    root: String,
    /// Whether the rules whose status is `preview` run, after `--preview`/`--no-preview`.
    preview: bool,
}

#[derive(Serialize)]
struct RulesDocument<'a> {
    schema_version: u32,
    config: ConfigView,
    rules: Vec<RuleView<'a>>,
}

pub fn run(args: &RulesArgs) -> Result<()> {
    let list = match &args.command {
        Some(RulesCommand::Export(export)) => return rules_export::run(export),
        Some(RulesCommand::New(new)) => return rules_new::run(new),
        Some(RulesCommand::List(list)) => list,
        None => &args.list,
    };
    let project = project::load(&list.project)?;
    let mut out = std::io::stdout().lock();
    if list.json {
        write_json(&project, list.all, &mut out)
    } else {
        write_table(&project, list.all, &mut out)
    }
}

fn write_json(project: &Project, all: bool, out: &mut impl Write) -> Result<()> {
    let document = RulesDocument {
        schema_version: SCHEMA_VERSION,
        config: ConfigView {
            path: project
                .loaded
                .source
                .as_ref()
                .map(|path| path.display().to_string()),
            root: project.loaded.root.display().to_string(),
            preview: project.loaded.config.preview,
        },
        rules: project
            .resolution
            .entries()
            .iter()
            .filter(|entry| all || entry.active)
            .map(RuleView::new)
            .collect(),
    };
    serde_json::to_writer(&mut *out, &document)?;
    writeln!(out)?;
    Ok(())
}

fn write_table(project: &Project, all: bool, out: &mut impl Write) -> Result<()> {
    let (loaded, resolution) = (&project.loaded, &project.resolution);
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
        .filter(|entry| all || entry.active)
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
