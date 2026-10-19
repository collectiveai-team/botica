//! `botica refs update`: follows the anchor of each external reference to the head of its
//! repository and proposes the commit and the lines, or writes them.

use std::collections::HashMap;
use std::fs;
use std::process::ExitCode;

use anyhow::{anyhow, bail, Context, Result};
use botica_catalog::{Origin, Reference, Topic, REFERENCES_DIR, TOPIC_FILE};

use crate::refs_collection::{label, Collection};
use crate::refs_edit::set_fields;

pub struct Update<'a> {
    pub topic: &'a str,
    pub write: bool,
}

/// What the head of the repository means for one reference.
enum Outcome {
    UpToDate,
    Moves(Move),
    Problem(String),
}

struct Move {
    rev: String,
    /// The new `lines`; absent when the reference points at a whole file or the lines stay.
    lines: Option<String>,
    text_changed: bool,
}

pub fn run(collection: &Collection, update: &Update) -> Result<ExitCode> {
    let topics = collection.loaded()?;
    let topic = topics.topic(update.topic).ok_or_else(|| {
        anyhow!(
            "no topic `{}` (`botica refs list` lists the topics)",
            update.topic
        )
    })?;
    if update.write && topic.origin == Origin::BuiltIn {
        bail!(
            "the topic `{}` is built in: its `rev` is edited in `catalog/references/` of botica",
            topic.id
        );
    }
    let mut heads = HashMap::new();
    let (mut moves, mut current, mut problems) = (Vec::new(), 0, 0);
    for (index, reference) in topic.references.iter().enumerate() {
        let Some(repo) = external_file(reference) else {
            continue;
        };
        let head = match heads.get(repo) {
            Some(head) => head,
            None => heads
                .entry(repo.to_owned())
                .or_insert(collection.head(repo).map_err(anyhow::Error::msg)?),
        };
        let name = format!("references[{index}] {}", label(reference));
        match outcome(collection, reference, head).map_err(anyhow::Error::msg)? {
            Outcome::UpToDate => {
                current += 1;
                println!("{name}: up to date");
            }
            Outcome::Moves(found) => {
                println!("{name}: {}", describe(reference, &found));
                moves.push((index, found));
            }
            Outcome::Problem(message) => {
                problems += 1;
                eprintln!("references[{index}] ({}): {message}", label(reference));
            }
        }
    }
    if update.write && !moves.is_empty() {
        write(collection, topic, &moves)?;
    }
    let verb = if update.write { "updated" } else { "to update" };
    println!(
        "{}: {} {verb}, {current} up to date, {problems} problem{}",
        topic.id,
        moves.len(),
        if problems == 1 { "" } else { "s" }
    );
    Ok(if problems == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

/// The repository of a reference that points at a file of another repository.
fn external_file(reference: &Reference) -> Option<&str> {
    reference.path.as_ref()?;
    reference.rev.as_ref()?;
    reference.repo.as_deref()
}

fn outcome(collection: &Collection, reference: &Reference, head: &str) -> Result<Outcome, String> {
    if reference.rev.as_deref() == Some(head) {
        return Ok(Outcome::UpToDate);
    }
    let moved = Reference {
        rev: Some(head.to_owned()),
        ..reference.clone()
    };
    if let Some(problem) = moved.problems().first() {
        return Err(format!(
            "the head of the repository is not usable: {problem}"
        ));
    }
    let path = reference.path.as_deref().unwrap_or_default();
    let old = collection.file_text(reference, path)?;
    let new = match collection.file_text(&moved, path) {
        Ok(new) => new,
        Err(message) => return Ok(Outcome::Problem(message)),
    };
    Ok(match relocate(reference, &old, &new, head) {
        Ok((lines, text_changed)) => Outcome::Moves(Move {
            rev: head.to_owned(),
            lines,
            text_changed,
        }),
        Err(message) => Outcome::Problem(message),
    })
}

/// The new `lines` of `reference` in `new`, the file at the commit `head`, when they change, and
/// whether the text they show is not the one of `old`.
fn relocate(
    reference: &Reference,
    old: &str,
    new: &str,
    head: &str,
) -> Result<(Option<String>, bool), String> {
    let Some((first, last)) = reference.line_range() else {
        if let Some(anchor) = reference.anchor.as_deref() {
            if !new.contains(anchor) {
                return Err(format!(
                    "the anchor `{anchor}` is not in the file at {head}"
                ));
            }
        }
        return Ok((None, old != new));
    };
    let Some(anchor) = reference.anchor.as_deref() else {
        return Err(
            "has `lines` and no `anchor`, so botica cannot follow them: add an `anchor`".to_owned(),
        );
    };
    reference.check(old)?;
    let offset = reference
        .excerpt(old)
        .lines()
        .position(|line| line.contains(anchor))
        .ok_or_else(|| format!("the anchor `{anchor}` spans more than one line: use one line"))?;
    let found: Vec<usize> = new
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(anchor))
        .map(|(position, _)| position)
        .collect();
    let position = match found.as_slice() {
        [] => {
            return Err(format!(
                "the anchor `{anchor}` is not in the file at {head}"
            ))
        }
        [one] => *one,
        several => {
            let at: Vec<String> = several.iter().map(|line| (line + 1).to_string()).collect();
            return Err(format!(
                "the anchor `{anchor}` is in {} places of the file (lines {}): make it longer",
                several.len(),
                at.join(", ")
            ));
        }
    };
    let Some(start) = position.checked_sub(offset) else {
        return Err("the lines would start before the file".to_owned());
    };
    let count = last - first + 1;
    let total = new.lines().count();
    if start + count > total {
        return Err(format!(
            "the lines would end past the end of the file, which has {total} lines"
        ));
    }
    let lines = if count == 1 {
        (start + 1).to_string()
    } else {
        format!("{}-{}", start + 1, start + count)
    };
    let moved = Reference {
        lines: Some(lines.clone()),
        ..reference.clone()
    };
    let text_changed = moved.excerpt(new) != reference.excerpt(old);
    let changed = (start + 1 != first).then_some(lines);
    Ok((changed, text_changed))
}

fn describe(reference: &Reference, found: &Move) -> String {
    let mut text = format!("rev -> {}", found.rev);
    if let (Some(new), Some(old)) = (&found.lines, &reference.lines) {
        text.push_str(&format!(", lines {old} -> {new}"));
    }
    if found.text_changed {
        text.push_str(", the text changed");
    }
    text
}

/// Rewrites the `rev` and the `lines` of the references of `moves`, in the `topic.toml` of the
/// project, and nothing else.
fn write(collection: &Collection, topic: &Topic, moves: &[(usize, Move)]) -> Result<()> {
    let file = collection
        .root()
        .join(REFERENCES_DIR)
        .join(&topic.id)
        .join(TOPIC_FILE);
    let mut text =
        fs::read_to_string(&file).with_context(|| format!("cannot read `{}`", file.display()))?;
    for (index, found) in moves {
        text = set_fields(&text, *index, &found.rev, found.lines.as_deref())
            .map_err(anyhow::Error::msg)?;
    }
    fs::write(&file, text).with_context(|| format!("cannot write `{}`", file.display()))
}
