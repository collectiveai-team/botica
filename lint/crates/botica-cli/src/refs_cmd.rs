use std::fmt::Write as _;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Result};
use botica_catalog::{Origin, Reference, Topic, REFERENCES_DIR};
use clap::{Args, Subcommand};

use crate::project::ProjectArgs;
use crate::refs_collection::{self, label, Collection};
use crate::{refs_check, refs_export};

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
    Check(CheckArgs),
    /// Print every topic as JSON, with the text of each file that its references show, for the
    /// pages of the site.
    Export(ExportArgs),
}

#[derive(Args)]
struct ProjectOnly {
    #[command(flatten)]
    project: ProjectArgs,
}

#[derive(Args)]
struct CacheArgs {
    /// Where the files of external references are kept, by repository and commit. Default:
    /// `botica/refs` under `XDG_CACHE_HOME`, `LOCALAPPDATA` (Windows) or `~/.cache`.
    #[arg(long)]
    cache_dir: Option<PathBuf>,
}

#[derive(Args)]
struct ShowArgs {
    /// A topic id, a rule (its slug, its code or an alias) or a standard (`CES-74`).
    key: String,
    #[command(flatten)]
    project: ProjectArgs,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Args)]
struct ExportArgs {
    #[command(flatten)]
    project: ProjectArgs,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Args)]
struct CheckArgs {
    /// Also fetch and check the references of the built-in topics, which botica checks in its own
    /// repository.
    #[arg(long)]
    all: bool,
    #[command(flatten)]
    project: ProjectArgs,
    #[command(flatten)]
    cache: CacheArgs,
}

pub fn run(args: &RefsArgs) -> Result<ExitCode> {
    match &args.command {
        RefsCommand::List(args) => list(&refs_collection::open(&args.project, None)?),
        RefsCommand::Show(args) => show(
            &refs_collection::open(&args.project, args.cache.cache_dir.clone())?,
            &args.key,
        ),
        RefsCommand::Export(args) => refs_export::run(&refs_collection::open(
            &args.project,
            args.cache.cache_dir.clone(),
        )?),
        RefsCommand::Check(args) => Ok(refs_check::run(
            &refs_collection::open(&args.project, args.cache.cache_dir.clone())?,
            args.all,
        )),
    }
}

fn list(collection: &Collection) -> Result<ExitCode> {
    let topics = collection.loaded()?;
    let mut out = std::io::stdout().lock();
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
    let from = match topic.origin {
        Origin::BuiltIn => "built-in",
        Origin::Project => "project",
    };
    let _ = writeln!(block, "  from: {from}");
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
        text.push_str(&reference_text(collection, reference)?);
    }
    Ok(text)
}

/// A page is its address and its note; a file is also the lines it shows. An external file has its
/// license and, on GitHub, its permalink.
fn reference_text(collection: &Collection, reference: &Reference) -> Result<String> {
    let mut head = format!("## {}\n\n{}\n\n", label(reference), reference.note);
    let Some(path) = &reference.path else {
        return Ok(head);
    };
    let mut meta = String::new();
    if let Some(license) = &reference.license {
        let _ = writeln!(meta, "license: {license}");
    }
    if let Some(link) = permalink(reference, path) {
        let _ = writeln!(meta, "{link}");
    }
    if !meta.is_empty() {
        head.push_str(&meta);
        head.push('\n');
    }
    let file = collection
        .file_text(reference, path)
        .map_err(anyhow::Error::msg)?;
    let mut shown = reference.excerpt(&file);
    if !shown.ends_with('\n') {
        shown.push('\n');
    }
    let fence = fence_for(&shown);
    Ok(format!("{head}{fence}\n{shown}{fence}\n\n"))
}

/// The address of the lines of a file of GitHub at its commit.
fn permalink(reference: &Reference, path: &str) -> Option<String> {
    let (repo, rev) = (reference.repo.as_deref()?, reference.rev.as_deref()?);
    repo.starts_with("github.com/").then(|| {
        let lines = match reference.line_range() {
            Some((first, last)) if first == last => format!("#L{first}"),
            Some((first, last)) => format!("#L{first}-L{last}"),
            None => String::new(),
        };
        format!("https://{repo}/blob/{rev}/{path}{lines}")
    })
}

/// A fence of backticks longer than any run of them in `text`, and of three at least: a Markdown
/// file with blocks of code cannot close it.
fn fence_for(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    "`".repeat(longest.max(2) + 1)
}
