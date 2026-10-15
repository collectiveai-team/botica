use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use botica_catalog::{Catalog, Rule, OWN_RULES_DIR};
use clap::{Args, ValueEnum};

const OWN_RULES_DESCRIPTION: &str =
    "What a run reports about itself: unreadable paths and suppression comments.";

#[derive(Args)]
pub struct ExportArgs {
    /// What to write.
    #[arg(long, value_enum)]
    format: Format,
    /// The directory to write to. It is created if it is missing, and no file in it is deleted.
    #[arg(long)]
    output: PathBuf,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    /// One page per rule and an index, for the standards site.
    Markdown,
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
    let catalog = Catalog::embedded()?;
    let pages = match args.format {
        Format::Markdown => render(&catalog),
    };
    write_pages(&args.output, &pages)
}

fn render(catalog: &Catalog) -> Vec<Page> {
    let groups = groups(catalog);
    let mut pages = vec![Page {
        path: PathBuf::from("index.md"),
        text: index(&groups),
    }];
    for group in &groups {
        for rule in &group.rules {
            pages.push(Page {
                path: Path::new(group.name).join(file_name(rule)),
                text: format!("{}\n{}", front_matter(group.name, rule), rule.readme),
            });
        }
    }
    pages
}

/// The code comes first, in lower case, so that a site which lists the pages by file name lists
/// the rules of a family in the order of their codes (a code ends in three digits).
fn file_name(rule: &Rule) -> String {
    format!("{}-{}.md", rule.code.to_lowercase(), rule.id)
}

fn groups(catalog: &Catalog) -> Vec<Group<'_>> {
    let mut by_name: BTreeMap<&str, Vec<&Rule>> = BTreeMap::new();
    for rule in catalog.rules() {
        let name = rule.family.as_deref().unwrap_or(OWN_RULES_DIR);
        by_name.entry(name).or_default().push(rule);
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

/// A JSON value is also a YAML one, so it quotes and escapes what the front matter holds.
fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("a rule field is JSON")
}

fn front_matter(group: &str, rule: &Rule) -> String {
    let mut lines = vec![
        format!("rule: {}", json(&rule.id)),
        format!("code: {}", json(&rule.code)),
    ];
    if let Some(family) = &rule.family {
        lines.push(format!("family: {}", json(family)));
    }
    if let Some(ces) = &rule.ces {
        lines.push(format!("ces: {}", json(ces)));
    }
    lines.extend([
        format!("summary: {}", json(&rule.summary)),
        format!("status: {}", json(&rule.status)),
        format!("since: {}", json(&rule.since)),
        format!("default-enabled: {}", rule.default_enabled),
        format!("default-severity: {}", json(&rule.default_severity)),
        format!("languages: {}", json(&rule.languages())),
        format!("tags: {}", json(&rule.tags)),
        format!("aliases: {}", json(&rule.aliases)),
        format!(
            "readme: {}",
            json(&format!("{group}/{}/README.md", rule.id))
        ),
    ]);
    format!("---\n{}\n---\n", lines.join("\n"))
}

fn index(groups: &[Group<'_>]) -> String {
    let mut text = String::from("# Rules\n\n");
    for group in groups {
        text.push_str(&format!(
            "## {}\n\n{}\n\n| Code | Rule | Status | Summary |\n|---|---|---|---|\n",
            group.name, group.description
        ));
        for rule in &group.rules {
            text.push_str(&format!(
                "| `{}` | [`{}`]({}/{}) | {} | {} |\n",
                rule.code,
                rule.id,
                group.name,
                file_name(rule),
                json(&rule.status).trim_matches('"'),
                rule.summary.replace('|', "\\|")
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

    fn catalog_with_summary(summary: &str) -> Catalog {
        let rule = format!(
            "id = \"log-no-print\"\ncode = \"BTLOG001\"\nsummary = {}\nstatus = \"stable\"\n\
             since = \"0.1.0\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\n\
             default-enabled = true\ndefault-severity = \"error\"\n\n\
             [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\n\
             file = \"python.yml\"\n",
            json(&summary)
        );
        Catalog::from_files(&[
            file(
                "logging/family.toml",
                "code = \"LOG\"\ndescription = \"d\"\n",
            ),
            file("logging/log-no-print/rule.toml", &rule),
            file("logging/log-no-print/README.md", "# r\n"),
            file("logging/log-no-print/python.yml", "id: log-no-print\n"),
        ])
        .expect("the catalog loads")
    }

    #[test]
    fn a_pipe_in_a_summary_does_not_split_the_cell_of_the_index() {
        let pages = render(&catalog_with_summary("a | b"));

        let index = &pages[0].text;
        assert!(index.contains("| stable | a \\| b |\n"), "{index}");
    }

    #[test]
    fn a_summary_with_quotes_stays_one_value_of_the_front_matter() {
        let pages = render(&catalog_with_summary("named \"{Name}Request\": yes"));

        let page = &pages[1].text;
        assert!(
            page.contains("summary: \"named \\\"{Name}Request\\\": yes\"\n"),
            "{page}"
        );
    }
}
