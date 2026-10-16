use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use botica_catalog::{Catalog, Rule, OWN_RULES_DIR};
use botica_config::Resolution;
use clap::{Args, ValueEnum};

use crate::agents_md;
use crate::catalog_json;
use crate::project::{self, ProjectArgs};
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
    /// it is deleted), and the file that holds the block for `agents-md`.
    #[arg(long)]
    output: PathBuf,
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
}

struct Page {
    path: PathBuf,
    text: String,
}

struct Group<'a> {
    name: &'a str,
    description: &'a str,
    rules: Vec<&'a Rule>,
}

pub fn run(args: &ExportArgs) -> Result<()> {
    if args.check && matches!(args.format, Format::Markdown | Format::Json) {
        bail!("--check applies to --format agents-md");
    }
    let catalog = Catalog::embedded()?;
    let pages = match args.format {
        Format::Markdown => render(&catalog),
        Format::Json => vec![Page {
            path: PathBuf::from("catalog.json"),
            text: catalog_json::render(&catalog),
        }],
        Format::AgentsMd => return export_agents_md(args),
    };
    write_pages(&args.output, &pages)
}

fn export_agents_md(args: &ExportArgs) -> Result<()> {
    let project = project::load(&args.project)?;
    let path = &args.output;
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read `{}`", path.display()))?;
    match updated_agents_md(args, &project.resolution, &text)? {
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
    resolution: &Resolution,
    text: &str,
) -> Result<Option<String>> {
    let path = args.output.display();
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

fn groups(catalog: &Catalog) -> Vec<Group<'_>> {
    let mut by_name: BTreeMap<&str, Vec<&Rule>> = BTreeMap::new();
    for rule in catalog.rules() {
        by_name.entry(group_of(rule)).or_default().push(rule);
    }
    by_name
        .into_iter()
        .map(|(name, mut rules)| {
            rules.sort_by(|a, b| a.code.cmp(&b.code));
            Group {
                name,
                description: describe(catalog, name),
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
            "## {}\n\n{}\n\n| Code | Rule | Status | Added in | Default |\n|---|---|---|---|---|\n",
            group.name, group.description
        ));
        for rule in &group.rules {
            text.push_str(&format!(
                "| `{}` | [`{}`]({}/{})<br>{} | {} | {} | {} |\n",
                rule.code,
                rule.id,
                group.name,
                file_name(rule),
                rule.summary.replace('|', "\\|"),
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
mod tests {
    use super::*;
    use botica_catalog::SourceFile;

    fn file(path: &str, text: &str) -> SourceFile {
        SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_pipe_in_a_summary_does_not_split_the_cell_of_the_index() {
        let rule = "id = \"log-no-print\"\ncode = \"BTLOG001\"\nsummary = \"a | b\"\n\
                    status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\n\
                    run-on = [\"cli\"]\ndefault-enabled = true\ndefault-severity = \"error\"\n\n\
                    [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\n\
                    file = \"python.yml\"\n";
        let catalog = Catalog::from_files(&[
            file(
                "logging/family.toml",
                "code = \"LOG\"\ndescription = \"d\"\n",
            ),
            file("logging/log-no-print/rule.toml", rule),
            file("logging/log-no-print/README.md", "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"),
            file("logging/log-no-print/python.yml", "id: log-no-print\n"),
        ])
        .expect("the catalog loads");

        let pages = render(&catalog);

        let index = &pages[0].text;
        assert!(index.contains("<br>a \\| b | stable |"), "{index}");
    }

    #[derive(clap::Parser)]
    struct Command {
        #[command(flatten)]
        export: ExportArgs,
    }

    fn args(extra: &[&str]) -> ExportArgs {
        use clap::Parser;
        let base = ["export", "--format", "agents-md", "--output", "AGENTS.md"];
        Command::parse_from(base.iter().chain(extra)).export
    }

    #[test]
    fn the_block_of_agents_md_links_each_rule_under_the_base_url() {
        let resolution = crate::agents_md::tests::resolution("");
        let text = "<!-- botica:rules -->\n<!-- /botica:rules -->\n";

        let updated = updated_agents_md(&args(&["--base-url", "https://x.org"]), &resolution, text)
            .expect("the markers are there")
            .expect("the block changes");

        assert!(
            updated.contains(" → https://x.org/rules/testing/bttest901-test-one/\n"),
            "{updated}"
        );
    }
}
