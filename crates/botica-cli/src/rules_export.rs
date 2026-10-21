use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use botica_catalog::{Catalog, Rule, OWN_RULES_DIR};
use botica_config::Resolution;
use botica_core::walk;
use clap::{Args, ValueEnum};

use crate::agents_md;
use crate::catalog_json;
use crate::project::{self, ProjectArgs};
use crate::rubric;
use crate::rule_page::{self, added_in, file_name, group_of, word};

const OWN_RULES_DESCRIPTION: &str =
    "What a run reports about itself: unreadable paths and suppression comments.";

const INDEX_INTRO: &str = "A rule is `stable`, `preview` (it runs only with `preview = true`), \
                           `deprecated` (it will be removed) or `removed` (documented for history).";

#[derive(Args)]
pub struct ExportArgs {
    /// What to write.
    #[arg(long, value_enum)]
    format: Format,
    /// Where to write: a directory for `markdown` and `json` (created if it is missing; no file in
    /// it is deleted), the file that holds the block for `agents-md`, and the rubric for `abide`
    /// (`.abide/rubric.json`, where abide reads it, by default).
    #[arg(long)]
    output: Option<PathBuf>,
    /// The file with the block of the judgment rules, whose lines the rubric of `abide` names.
    #[arg(long, default_value = rubric::DEFAULT_AGENTS_MD)]
    agents_md: PathBuf,
    /// Write nothing, and fail when what the export would write differs from what is there.
    #[arg(long)]
    check: bool,
    /// The address of the standards site, where `agents-md` links each rule.
    #[arg(long, default_value = agents_md::SITE)]
    base_url: String,
    #[command(flatten)]
    project: ProjectArgs,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    /// One page per rule and an index, for the standards site.
    Markdown,
    /// The whole catalog in one file, `catalog.json`.
    Json,
    /// The judgment rules that the project runs, between `<!-- botica:rules -->` and
    /// `<!-- /botica:rules -->` in the file of `--output`.
    AgentsMd,
    /// The rubric of abide: the judgment rules that the project runs, with their questions and
    /// the files they apply to.
    Abide,
}

struct Page {
    path: PathBuf,
    text: String,
}

struct Group<'a> {
    name: &'a str,
    description: String,
    rules: Vec<&'a Rule>,
}

pub fn run(args: &ExportArgs) -> Result<()> {
    if args.check && matches!(args.format, Format::Markdown | Format::Json) {
        bail!("--check applies to --format agents-md and abide");
    }
    if matches!(args.format, Format::Abide) {
        return export_abide(args);
    }
    let output = args
        .output
        .as_deref()
        .with_context(|| format!("--format {} needs --output", args.format.name()))?;
    let project = project::load(&args.project)?;
    let catalog = &project.catalog;
    let pages = match args.format {
        Format::Markdown => render(catalog),
        Format::Json => vec![Page {
            path: PathBuf::from("catalog.json"),
            text: catalog_json::render(catalog),
        }],
        Format::AgentsMd => return export_agents_md(args, output),
        Format::Abide => unreachable!("the rubric is exported above"),
    };
    write_pages(output, &pages)
}

/// `botica rules export --format <name> --check` for an export that a recipe asks for, with the
/// files that the export has by default: `AGENTS.md` and `.abide/rubric.json`.
pub(crate) fn check_export(name: &str, project: &ProjectArgs) -> Result<()> {
    recipe_export(name, project, true)
}

/// `botica rules export --format <name>` for an export that a recipe asks for, written to the
/// files that the export has by default.
pub(crate) fn write_export(name: &str, project: &ProjectArgs) -> Result<()> {
    recipe_export(name, project, false)
}

fn recipe_export(name: &str, project: &ProjectArgs, check: bool) -> Result<()> {
    let (format, output) = match name {
        "agents-md" => (
            Format::AgentsMd,
            Some(PathBuf::from(rubric::DEFAULT_AGENTS_MD)),
        ),
        "abide" => (Format::Abide, None),
        other => bail!("unknown export `{other}`"),
    };
    run(&ExportArgs {
        format,
        output,
        agents_md: PathBuf::from(rubric::DEFAULT_AGENTS_MD),
        check,
        base_url: agents_md::SITE.to_owned(),
        project: project.clone(),
    })
}

impl Format {
    fn name(self) -> String {
        self.to_possible_value()
            .map(|value| value.get_name().to_owned())
            .unwrap_or_default()
    }
}

fn export_agents_md(args: &ExportArgs, path: &Path) -> Result<()> {
    let project = project::load(&args.project)?;
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read `{}`", path.display()))?;
    match updated_agents_md(args, path, &project.resolution, &text)? {
        Some(updated) => {
            fs::write(path, updated).with_context(|| format!("cannot write `{}`", path.display()))
        }
        None => Ok(()),
    }
}

/// The text of the file of `--output` with the block of the project, or `None` when the block is
/// already there. With `--check` a block that differs is an error.
fn updated_agents_md(
    args: &ExportArgs,
    path: &Path,
    resolution: &Resolution,
    text: &str,
) -> Result<Option<String>> {
    let path = path.display();
    let block = agents_md::block(resolution, &args.base_url);
    let replaced =
        agents_md::replace(text, &block).map_err(|reason| anyhow!("`{path}` {reason}"))?;
    if replaced == text {
        return Ok(None);
    }
    if args.check {
        bail!(
            "the block of `{path}` differs from the catalog and the configuration: run \
             `botica rules export --format agents-md --output {path}`"
        );
    }
    Ok(Some(replaced))
}

/// The files of the project from its root, minus what its `extend-exclude` leaves out of a walk.
fn project_files(root: &Path, resolution: &Arc<Resolution>) -> Vec<String> {
    let exclusion = project::exclusion(Arc::clone(resolution), None);
    walk::files(root, exclusion.as_ref())
}

/// Writes the rubric of the project, unless the one there has its sources and rules; with
/// `--check` a rubric that differs or is missing is an error.
fn export_abide(args: &ExportArgs) -> Result<()> {
    let project = project::load(&args.project)?;
    let root = project.loaded.root.clone();
    let resolution = Arc::new(project.resolution);
    let files = walk::files(&root, None);
    let linted = project_files(&root, &resolution);
    let agents_md = fs::read_to_string(&args.agents_md)
        .with_context(|| format!("cannot read `{}`", args.agents_md.display()))?;
    let read = |path: &str| fs::read_to_string(root.join(path)).ok();
    let agents_md_path = args.agents_md.to_string_lossy();
    let inputs = rubric::Inputs {
        resolution: &resolution,
        files: &files,
        linted: &linted,
        read: &read,
        agents_md_path: &agents_md_path,
        agents_md: &agents_md,
    };
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let rubric = rubric::rubric(&inputs, rubric::rfc3339(now)).map_err(|reason| anyhow!(reason))?;
    let output = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(rubric::DEFAULT_OUTPUT));
    let existing = fs::read_to_string(&output).ok();
    if existing.is_some_and(|text| rubric::unchanged(&text, &rubric)) {
        return Ok(());
    }
    if args.check {
        bail!(
            "`{}` differs from the catalog, the configuration or the `ignore-file` comments of \
             the project: run `botica rules export --format abide`",
            output.display()
        );
    }
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create `{}`", parent.display()))?;
    }
    fs::write(&output, rubric::render(&rubric))
        .with_context(|| format!("cannot write `{}`", output.display()))
}

fn render(catalog: &Catalog) -> Vec<Page> {
    let groups = groups(catalog);
    let mut pages = vec![Page {
        path: PathBuf::from("index.md"),
        text: index(catalog, &groups),
    }];
    for group in &groups {
        for rule in &group.rules {
            pages.push(Page {
                path: Path::new(group.name).join(file_name(rule)),
                text: rule_page::page(catalog, rule),
            });
        }
    }
    pages
}

/// The groups of the built-in catalog by name, then those of the catalogs of the project. A
/// family of a project that extends a built-in one has its rules in the group of that one, after
/// the built-in rules.
fn groups(catalog: &Catalog) -> Vec<Group<'_>> {
    let mut by_name: BTreeMap<(bool, &str), Vec<&Rule>> = BTreeMap::new();
    for rule in catalog.rules() {
        let own_group = rule.catalog.is_some() && rule.extends.is_none();
        by_name
            .entry((own_group, group_of(rule)))
            .or_default()
            .push(rule);
    }
    by_name
        .into_iter()
        .map(|((project, name), mut rules)| {
            rules.sort_by(|a, b| {
                (a.catalog.is_some(), &a.code).cmp(&(b.catalog.is_some(), &b.code))
            });
            Group {
                name,
                description: if project {
                    format!("Rules of the catalog `{name}` of the project.")
                } else {
                    describe(catalog, name).to_owned()
                },
                rules,
            }
        })
        .collect()
}

fn describe<'a>(catalog: &'a Catalog, group: &str) -> &'a str {
    if group == OWN_RULES_DIR {
        return OWN_RULES_DESCRIPTION;
    }
    catalog
        .families()
        .iter()
        .find(|family| family.name == group)
        .map_or("", |family| family.description.as_str())
}

fn index(catalog: &Catalog, groups: &[Group<'_>]) -> String {
    let mut text = format!("# Rules\n\n{INDEX_INTRO}\n\n");
    for group in groups {
        text.push_str(&format!(
            "## {}\n\n{}\n\n| Code | Rule | Standard | Status | Added in | Default |\n|---|---|---|---|---|---|\n",
            group.name, group.description
        ));
        for rule in &group.rules {
            let tags = if rule.tags.is_empty() {
                String::new()
            } else {
                format!("<br><small>{}</small>", rule.tags.join(", "))
            };
            text.push_str(&format!(
                "| `{}` | [`{}`]({}/{})<br>{}{} | {} | {} | {} | {} |\n",
                rule.code,
                rule.id,
                group.name,
                file_name(rule),
                rule.summary.replace('|', "\\|"),
                tags,
                rule.standard.as_deref().unwrap_or_default(),
                word(&rule.status),
                added_in(catalog, rule),
                if rule.default_enabled { "on" } else { "off" }
            ));
        }
        text.push('\n');
    }
    text
}

fn write_pages(output: &Path, pages: &[Page]) -> Result<()> {
    for page in pages {
        let path = output.join(&page.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("cannot create `{}`", parent.display()))?;
        }
        fs::write(&path, &page.text)
            .with_context(|| format!("cannot write `{}`", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
