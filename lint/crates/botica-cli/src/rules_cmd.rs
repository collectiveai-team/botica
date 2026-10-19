use std::io::Write;
use std::process::ExitCode;

use anyhow::Result;
use botica_catalog::{Kind, Rule};
use botica_config::Resolution;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::extension_pool::Launcher;
use crate::judgment;
use crate::project::{self, Project, ProjectArgs};
use crate::rule_view::{Programs, RuleView};
use crate::rules_conform::{self, ConformArgs};
use crate::rules_eval::{self, EvalArgs};
use crate::rules_export::{self, ExportArgs};
use crate::rules_new::{self, NewArgs};
use crate::rules_test::{self, TestArgs};

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
    /// Have a judge answer the fixtures of each judgment rule of the catalog of a checkout and
    /// compare its verdicts with the `expected.json` of the rule.
    Test(TestArgs),
    /// Run each rule of a catalog over its fixtures, with and without each optimisation, and
    /// compare the findings with the `expected.json` of the rule.
    Conform(ConformArgs),
    /// Run one deterministic rule over real code, whatever the configuration selects, and list
    /// what it finds, for a review of its false positives. The findings do not fail the run.
    Eval(EvalArgs),
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

pub fn run(args: &RulesArgs) -> Result<ExitCode> {
    let list = match &args.command {
        Some(RulesCommand::Export(export)) => return rules_export::run(export).map(success),
        Some(RulesCommand::New(new)) => return rules_new::run(new).map(success),
        Some(RulesCommand::Test(test)) => return rules_test::run(test),
        Some(RulesCommand::Conform(conform)) => return rules_conform::run(conform),
        Some(RulesCommand::Eval(eval)) => return rules_eval::run(eval),
        Some(RulesCommand::List(list)) => list,
        None => &args.list,
    };
    let project = project::load(&list.project)?;
    let mut out = std::io::stdout().lock();
    if list.json {
        write_json(&project, list.all, &mut out)?;
    } else {
        write_table(&project, list.all, &mut out)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn success(_: ()) -> ExitCode {
    ExitCode::SUCCESS
}

fn programs(project: &Project) -> Programs<'_> {
    Programs {
        launcher: Launcher::of(project),
        root: &project.loaded.root,
    }
}

fn write_json(project: &Project, all: bool, out: &mut impl Write) -> Result<()> {
    let programs = programs(project);
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
            .map(|entry| {
                RuleView::new(
                    entry,
                    &programs,
                    judgment::covering(&project.resolution, &entry.rule),
                )
            })
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
            "{:<12} {:<22} {:<9} {}{}",
            entry.rule.code,
            entry.rule.id,
            state,
            entry.reason,
            judged(resolution, &entry.rule)
        )?;
    }
    write_outside(project, out)
}

/// What the listing adds to a judgment rule: that a judge evaluates it, and the active rules that
/// cover it, which leave it out of the exports.
fn judged(resolution: &Resolution, rule: &Rule) -> String {
    if !rule.is_judgment() {
        return String::new();
    }
    let covering = judgment::covering(resolution, rule);
    if covering.is_empty() {
        return "; judged outside lint".to_owned();
    }
    let named: Vec<String> = covering.iter().map(|slug| format!("`{slug}`")).collect();
    format!(
        "; judged outside lint, covered by {}, left out of the exports",
        named.join(", ")
    )
}

/// The programs that `botica lint` starts for the active extension rules, and the tools that
/// enforce the active external rules.
fn write_outside(project: &Project, out: &mut impl Write) -> Result<()> {
    let programs = programs(project);
    let mut runs = Vec::new();
    let mut tools = Vec::new();
    for entry in project.resolution.entries().iter().filter(|e| e.active) {
        let rule = &entry.rule;
        for implementation in &rule.implementations {
            if let Some(argv) = programs.runs(rule, implementation) {
                runs.push(format!("  {}  {}", rule.id, argv.join(" ")));
            }
            if let (Kind::External, Some(tool)) = (implementation.kind, &implementation.tool) {
                tools.push(format!("  {}  {tool}", rule.id));
            }
        }
    }
    for (title, lines) in [
        ("programs that `botica lint` runs", runs),
        ("tools that enforce rules outside `botica lint`", tools),
    ] {
        if !lines.is_empty() {
            writeln!(out, "\n{title}:\n{}", lines.join("\n"))?;
        }
    }
    Ok(())
}
