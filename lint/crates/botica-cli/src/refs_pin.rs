//! `botica refs pin`: a permalink of GitHub becomes a reference of a topic of the project.

use std::fmt::Write as _;
use std::fs;
use std::io::ErrorKind;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use botica_catalog::{is_kebab_case, Origin, Reference, REFERENCES_DIR, TOPIC_FILE};

use crate::refs_collection::{label, Collection};

pub struct Pin<'a> {
    pub permalink: &'a str,
    pub topic: &'a str,
    pub license: &'a str,
    pub note: &'a str,
    pub title: Option<&'a str>,
}

const SHAPE: &str = "https://github.com/<owner>/<repo>/blob/<commit>/<path>, with #L<line> or \
                     #L<first>-L<last> for the lines";

pub fn run(collection: &Collection, pin: &Pin) -> Result<ExitCode> {
    if !is_kebab_case(pin.topic) {
        bail!("`{}` must be a kebab-case topic id", pin.topic);
    }
    let mut reference = reference_of(pin)?;
    let problems = reference.problems();
    if !problems.is_empty() {
        bail!("{}", problems.join("; "));
    }
    let path = reference.path.clone().unwrap_or_default();
    let text = collection
        .file_text(&reference, &path)
        .map_err(anyhow::Error::msg)?;
    reference.check(&text).map_err(anyhow::Error::msg)?;
    if reference.lines.is_some() {
        reference.anchor = first_line_with_text(&reference.excerpt(&text));
    }
    let block = block(&reference);
    write_topic(collection, pin, &block)?;
    if let Some(advice) = reference.suggestion() {
        eprintln!("warning: {advice}");
    }
    println!("pinned {} in {}", label(&reference), pin.topic);
    Ok(ExitCode::SUCCESS)
}

/// The reference that the permalink names, with the license and the note of the command.
fn reference_of(pin: &Pin) -> Result<Reference> {
    let link = pin.permalink;
    let (address, fragment) = match link.split_once('#') {
        Some((address, fragment)) => (address, Some(fragment)),
        None => (link, None),
    };
    let address = address.split('?').next().unwrap_or(address);
    let not_a_permalink = || anyhow::anyhow!("`{link}` is not a permalink of github.com: {SHAPE}");
    let rest = address
        .strip_prefix("https://github.com/")
        .ok_or_else(not_a_permalink)?;
    let parts: Vec<&str> = rest.splitn(5, '/').collect();
    let [owner, repo, "blob", rev, path] = parts.as_slice() else {
        return Err(not_a_permalink());
    };
    let lines = fragment.map(lines_of).transpose()?;
    Ok(Reference {
        repo: Some(format!("github.com/{owner}/{repo}")),
        rev: Some((*rev).to_owned()),
        license: Some(pin.license.to_owned()),
        path: Some((*path).to_owned()),
        lines,
        note: pin.note.to_owned(),
        ..Reference::default()
    })
}

/// `L10-L40` is `10-40` and `L10` is `10`.
fn lines_of(fragment: &str) -> Result<String> {
    let wrong = || anyhow::anyhow!("`#{fragment}` is not lines: use #L<line> or #L<first>-L<last>");
    let body = fragment.strip_prefix('L').ok_or_else(wrong)?;
    let (first, last) = match body.split_once('-') {
        Some((first, last)) => (first, last.strip_prefix('L').ok_or_else(wrong)?),
        None => (body, body),
    };
    let number = |digits: &str| digits.parse::<usize>().map_err(|_| wrong());
    let (first, last) = (number(first)?, number(last)?);
    let lines = if first == last {
        first.to_string()
    } else {
        format!("{first}-{last}")
    };
    let probe = Reference {
        lines: Some(lines.clone()),
        ..Reference::default()
    };
    if probe.line_range().is_none() {
        return Err(wrong());
    }
    Ok(lines)
}

/// The first line that has text, without its indentation: what `refs update` looks for.
fn first_line_with_text(excerpt: &str) -> Option<String> {
    excerpt
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

fn quoted(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

fn block(reference: &Reference) -> String {
    let mut block = String::from("[[references]]\n");
    let fields = [
        ("repo", &reference.repo),
        ("rev", &reference.rev),
        ("license", &reference.license),
        ("path", &reference.path),
        ("lines", &reference.lines),
        ("anchor", &reference.anchor),
    ];
    for (name, value) in fields {
        if let Some(value) = value {
            let _ = writeln!(block, "{name} = {}", quoted(value));
        }
    }
    let _ = writeln!(block, "note = {}", quoted(&reference.note));
    block
}

/// Adds `block` to the topic of the project, which `--title` creates when it does not exist.
fn write_topic(collection: &Collection, pin: &Pin, block: &str) -> Result<()> {
    let dir = collection.root().join(REFERENCES_DIR).join(pin.topic);
    let file = dir.join(TOPIC_FILE);
    match fs::read_to_string(&file) {
        Ok(mut text) => {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push('\n');
            text.push_str(block);
            fs::write(&file, text).with_context(|| format!("cannot write `{}`", file.display()))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => create_topic(collection, pin, block),
        Err(error) => Err(error).with_context(|| format!("cannot read `{}`", file.display())),
    }
}

fn create_topic(collection: &Collection, pin: &Pin, block: &str) -> Result<()> {
    let built_in = collection
        .topics
        .as_ref()
        .ok()
        .and_then(|topics| topics.topic(pin.topic))
        .is_some_and(|topic| topic.origin == Origin::BuiltIn);
    if built_in {
        bail!(
            "the topic `{}` is built in: pin the reference to a topic of {REFERENCES_DIR}/",
            pin.topic
        );
    }
    let Some(title) = pin.title else {
        bail!(
            "no topic `{}` in {REFERENCES_DIR}/ (`--title` creates it)",
            pin.topic
        );
    };
    let dir = collection.root().join(REFERENCES_DIR).join(pin.topic);
    fs::create_dir_all(&dir).with_context(|| format!("cannot create `{}`", dir.display()))?;
    let toml = format!("title = {}\n\n{block}", quoted(title));
    fs::write(dir.join(TOPIC_FILE), toml).context("cannot write the topic")?;
    fs::write(dir.join("README.md"), format!("# {title}\n")).context("cannot write the README")
}
