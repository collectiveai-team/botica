use std::fmt::Write as _;
use std::io::Write;
use std::process::ExitCode;

use anyhow::{bail, Result};
use botica_catalog::{Reference, Topic, REFERENCES_DIR};
use clap::{Args, Subcommand};

use crate::project::ProjectArgs;
use crate::refs_check;
use crate::refs_collection::{self, label, read_text, Collection};

#[derive(Args)]
pub struct RefsArgs {
    #[command(subcommand)]
    command: RefsCommand,
}

#[derive(Subcommand)]
enum RefsCommand {
    /// List the topics of `references/`, with their references and the rules and standards that
    /// cite them.
    List(ProjectOnly),
    /// Print the references of a topic, or of the topics that a rule or a standard cites, with
    /// their notes and the lines of the files they point at.
    Show(ShowArgs),
    /// Fail when a reference no longer resolves or something cites a topic that does not exist.
    Check(ProjectOnly),
}

#[derive(Args)]
struct ProjectOnly {
    #[command(flatten)]
    project: ProjectArgs,
}

#[derive(Args)]
struct ShowArgs {
    /// A topic id, a rule (its slug, its code or an alias) or a standard (`CES-74`).
    key: String,
    #[command(flatten)]
    project: ProjectArgs,
}

pub fn run(args: &RefsArgs) -> Result<ExitCode> {
    match &args.command {
        RefsCommand::List(args) => list(&refs_collection::open(&args.project)?),
        RefsCommand::Show(args) => show(&refs_collection::open(&args.project)?, &args.key),
        RefsCommand::Check(args) => Ok(refs_check::run(&refs_collection::open(&args.project)?)),
    }
}

fn list(collection: &Collection) -> Result<ExitCode> {
    let topics = collection.loaded()?;
    let mut out = std::io::stdout().lock();
    if topics.topics().is_empty() {
        writeln!(out, "no topics in {REFERENCES_DIR}/")?;
        return Ok(ExitCode::SUCCESS);
    }
    let blocks: Vec<String> = topics
        .topics()
        .iter()
        .map(|topic| list_block(collection, topic))
        .collect();
    write!(out, "{}", blocks.join("\n"))?;
    Ok(ExitCode::SUCCESS)
}

fn list_block(collection: &Collection, topic: &Topic) -> String {
    let mut block = format!("{}: {}\n", topic.id, topic.title);
    for reference in &topic.references {
        let _ = writeln!(block, "  {}  {}", label(reference), reference.note);
    }
    let citers = collection.cited_by(&topic.id);
    let cited = if citers.is_empty() {
        "nothing".to_owned()
    } else {
        citers.join(", ")
    };
    let _ = writeln!(block, "  cited by: {cited}");
    block
}

/// The topics that `key` names: the topic itself, or the ones a rule or a standard cites.
fn topics_of<'a>(collection: &'a Collection, key: &str) -> Result<Vec<&'a Topic>> {
    let topics = collection.loaded()?;
    if let Some(topic) = topics.topic(key) {
        return Ok(vec![topic]);
    }
    let (kind, name, cited) = if let Some(rule) = collection.project.catalog.resolve(key) {
        ("rule", rule.id.as_str(), &rule.references)
    } else if let Some(standard) = collection.standards.iter().find(|s| s.id == key) {
        ("standard", standard.id.as_str(), &standard.references)
    } else {
        bail!("no topic, rule or standard `{key}` (`botica refs list` lists the topics)");
    };
    if cited.is_empty() {
        bail!("{kind} {name} cites no topic");
    }
    cited
        .iter()
        .map(|id| {
            topics.topic(id).ok_or_else(|| {
                anyhow::anyhow!(
                    "the topic `{id}`, cited by {kind} {name}, is not in {REFERENCES_DIR}/ of this project"
                )
            })
        })
        .collect()
}

fn show(collection: &Collection, key: &str) -> Result<ExitCode> {
    let mut text = String::new();
    for topic in topics_of(collection, key)? {
        text.push_str(&topic_text(collection, topic)?);
    }
    write!(std::io::stdout().lock(), "{text}")?;
    Ok(ExitCode::SUCCESS)
}

fn topic_text(collection: &Collection, topic: &Topic) -> Result<String> {
    let mut text = format!(
        "# {}: {}\n\n{}\n\n",
        topic.id,
        topic.title,
        topic.readme.trim_end()
    );
    for reference in &topic.references {
        text.push_str(&reference_text(collection, topic, reference)?);
    }
    Ok(text)
}

/// A page is its address and its note; a file is also the lines it shows.
fn reference_text(collection: &Collection, topic: &Topic, reference: &Reference) -> Result<String> {
    let head = format!("## {}\n\n{}\n\n", label(reference), reference.note);
    let Some(path) = &reference.path else {
        return Ok(head);
    };
    if reference.is_external() {
        bail!(
            "external references are not supported yet: the topic `{}` points at {}",
            topic.id,
            label(reference)
        );
    }
    let file = read_text(collection.root(), path).map_err(anyhow::Error::msg)?;
    let mut shown = reference.excerpt(&file);
    if !shown.ends_with('\n') {
        shown.push('\n');
    }
    let fence = fence_for(&shown);
    Ok(format!("{head}{fence}\n{shown}{fence}\n\n"))
}

/// A fence of backticks longer than any run of them in `text`, and of three at least: a Markdown
/// file with blocks of code cannot close it.
fn fence_for(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}
