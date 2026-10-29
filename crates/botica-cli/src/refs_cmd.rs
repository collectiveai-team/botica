use std::fmt::Write as _;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Result};
use botica_catalog::{Origin, Reference, Topic, REFERENCES_DIR};
use clap::{Args, Subcommand};

use crate::project::ProjectArgs;
use crate::refs_collection::{self, label, Collection};
use crate::refs_pin::{self, Pin};
use crate::refs_update::{self, Update};
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
    /// Add the file of a permalink of GitHub to a topic of the project as a reference, with the
    /// lines of the permalink and the first of them as its anchor.
    Pin(PinArgs),
    /// Find the anchor of each external reference of a topic at the head of its repository and
    /// print the commit and the lines it proposes.
    Update(UpdateArgs),
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
struct PinArgs {
    /// `https://github.com/<owner>/<repo>/blob/<commit>/<path>#L10-L40`: press `y` on GitHub to
    /// get the address of the commit.
    permalink: String,
    /// The topic of `references/` that takes the reference.
    #[arg(long)]
    topic: String,
    /// The license of the file, as an SPDX id.
    #[arg(long)]
    license: String,
    /// What the reference shows.
    #[arg(long)]
    note: String,
    /// Create the topic, with this title, when it does not exist.
    #[arg(long)]
    title: Option<String>,
    #[command(flatten)]
    project: ProjectArgs,
    #[command(flatten)]
    cache: CacheArgs,
}

#[derive(Args)]
struct UpdateArgs {
    /// The topic whose references are followed to the head of their repositories.
    topic: String,
    /// Write the proposed `rev` and `lines` to the `topic.toml` of the project.
    #[arg(long)]
    write: bool,
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
        RefsCommand::Pin(args) => refs_pin::run(
            &refs_collection::open(&args.project, args.cache.cache_dir.clone())?,
            &Pin {
                permalink: &args.permalink,
                topic: &args.topic,
                license: &args.license,
                note: &args.note,
                title: args.title.as_deref(),
            },
        ),
        RefsCommand::Update(args) => refs_update::run(
            &refs_collection::open(&args.project, args.cache.cache_dir.clone())?,
            &Update {
                topic: &args.topic,
                write: args.write,
            },
        ),
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
    let catalog = &collection.project.catalog;
    let (kind, name, cited) = if let Some(rule) = catalog.resolve(key) {
        if let Some(warning) = catalog.retired_warning(key) {
            eprintln!("botica: warning: {warning}");
        }
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

/// A page is its address and its note; a file is also the lines it shows. An external file has its
/// license and, on GitHub, its permalink.
fn reference_text(collection: &Collection, topic: &Topic, reference: &Reference) -> Result<String> {
    let mut head = format!("## {}\n\n{}\n\n", label(reference), reference.note);
    let Some(file) = collection
        .reference_text(topic, reference)
        .map_err(anyhow::Error::msg)?
    else {
        if let Some(quote) = &reference.quote {
            head.push_str(&block_quote(quote));
            head.push('\n');
        }
        return Ok(head);
    };
    let mut meta = String::new();
    if let Some(license) = &reference.license {
        let _ = writeln!(meta, "license: {license}");
    }
    if let Some(link) = reference
        .path
        .as_deref()
        .and_then(|path| permalink(reference, path))
    {
        let _ = writeln!(meta, "{link}");
    }
    if !meta.is_empty() {
        head.push_str(&meta);
        head.push('\n');
    }
    let mut shown = reference.excerpt(&file);
    if !shown.ends_with('\n') {
        shown.push('\n');
    }
    let fence = fence_for(&shown);
    Ok(format!("{head}{fence}\n{shown}{fence}\n\n"))
}

/// `text` as a Markdown block quote, a line at a time.
fn block_quote(text: &str) -> String {
    text.trim()
        .lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_owned()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
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
