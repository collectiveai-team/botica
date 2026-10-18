//! What `botica recipe install` does for a file and for a section: it adds what is missing and
//! never edits a file that exists, except to append a section.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result};

/// What happened to a step.
pub enum Outcome {
    Added(Option<&'static str>),
    Present,
    Pending(String),
    Failed(String),
}

impl Outcome {
    /// The end of the line of the report, after `kind id: `.
    pub fn text(&self) -> String {
        match self {
            Self::Added(None) => "added".to_owned(),
            Self::Added(Some(detail)) => format!("added: {detail}"),
            Self::Present => "ok".to_owned(),
            Self::Pending(reason) => format!("pending: {reason}"),
            Self::Failed(reason) => format!("failed: {reason}"),
        }
    }
}

/// Where a template is, for the message of a pending file.
pub struct Template<'a> {
    pub text: &'a str,
    pub path: &'a str,
    pub recipe: &'a str,
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn create(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create `{}`", parent.display()))?;
    }
    fs::write(path, text).with_context(|| format!("cannot write `{}`", path.display()))
}

/// Writes the file from the template when the path does not exist. A path that exists is pending
/// unless it already has the text of the template.
pub fn add_file(root: &Path, path: &str, template: &Template) -> Result<Outcome> {
    let full = root.join(path);
    if !exists(&full) {
        create(&full, template.text)?;
        return Ok(Outcome::Added(None));
    }
    Ok(match fs::read_to_string(&full) {
        Ok(text) if text == template.text => Outcome::Present,
        _ => Outcome::Pending(format!(
            "exists and differs from the template `{}` of `{}`",
            template.path, template.recipe
        )),
    })
}

/// Writes the file with the section when the path does not exist, and appends the section after a
/// blank line to a file without the marker.
pub fn add_section(root: &Path, path: &str, marker: &str, template: &str) -> Result<Outcome> {
    let full = root.join(path);
    if !exists(&full) {
        create(&full, template)?;
        return Ok(Outcome::Added(Some("created the file")));
    }
    if !full.is_file() {
        return Ok(Outcome::Pending("is not a file".to_owned()));
    }
    let text = match fs::read_to_string(&full) {
        Ok(text) => text,
        Err(error) => return Ok(Outcome::Pending(format!("cannot read the file: {error}"))),
    };
    if text.contains(marker) {
        return Ok(Outcome::Present);
    }
    let separator = match text.as_str() {
        "" => "",
        text if text.ends_with('\n') => "\n",
        _ => "\n\n",
    };
    let mut file = OpenOptions::new()
        .append(true)
        .open(&full)
        .with_context(|| format!("cannot open `{}`", full.display()))?;
    write!(file, "{separator}{template}")
        .with_context(|| format!("cannot write `{}`", full.display()))?;
    Ok(Outcome::Added(Some("appended to the file")))
}
