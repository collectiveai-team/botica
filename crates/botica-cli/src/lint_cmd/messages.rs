//! `botica lint --commit-msg`, `--commits` and `--pr-title` (ADR 0010): the rules of scope
//! `commit` over commit messages, and no file.

use super::{git, LintArgs};
use anyhow::{Context, Result};
use botica_catalog::Scope;
use botica_config::Resolution;
use botica_core::commit::{without_comments, Message};
use botica_core::{registry, Diagnostic, Severity};

/// A message and where its findings are.
struct Read {
    place: String,
    text: String,
}

/// Whether the arguments ask for messages instead of files.
pub(super) fn asked(args: &LintArgs) -> bool {
    !args.commit_msg.is_empty() || args.commits.is_some() || args.pr_title.is_some()
}

fn read(args: &LintArgs) -> Result<Vec<Read>> {
    let mut messages = Vec::new();
    for path in &args.commit_msg {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read `{}`", path.display()))?;
        messages.push(Read {
            place: path.display().to_string(),
            text: without_comments(&text),
        });
    }
    if let Some(range) = &args.commits {
        messages.extend(git::commits(range)?.into_iter().map(|(sha, text)| Read {
            place: format!("commit {sha}"),
            text,
        }));
    }
    if let Some(title) = &args.pr_title {
        messages.push(Read {
            place: "pull request title".to_owned(),
            text: title.clone(),
        });
    }
    Ok(messages)
}

/// The findings of the active rules of scope `commit` in the messages the arguments name, at the
/// severity the configuration gives each rule.
pub(super) fn findings(args: &LintArgs, resolution: &Resolution) -> Result<Vec<Diagnostic>> {
    let rules: Vec<_> = resolution
        .entries()
        .iter()
        .filter(|entry| entry.active && entry.rule.scope == Some(Scope::Commit))
        .filter_map(|entry| {
            registry::build_message(&entry.rule.id, &entry.options)
                .map(|rule| (rule, Severity::from(entry.severity)))
        })
        .collect();
    let mut found = Vec::new();
    for read in read(args)? {
        let message = Message {
            place: &read.place,
            text: &read.text,
        };
        for (rule, severity) in &rules {
            let start = found.len();
            rule.check(&message, &mut found);
            for finding in &mut found[start..] {
                finding.severity = *severity;
            }
        }
    }
    Ok(found)
}
