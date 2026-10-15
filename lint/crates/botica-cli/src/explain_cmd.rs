use std::io::Write;

use anyhow::{bail, Result};
use botica_catalog::{Severity, Status};
use botica_config::{Entry, IgnoreOrigin};
use botica_core::registry;
use clap::Args;

use crate::project::{self, ProjectArgs};

#[derive(Args)]
pub struct ExplainArgs {
    /// The rule: its slug, its code or an alias.
    id: String,
    #[command(flatten)]
    project: ProjectArgs,
}

pub fn run(args: &ExplainArgs) -> Result<()> {
    let project = project::load(&args.project)?;
    let Some(entry) = project.resolution.entry(&args.id) else {
        bail!(
            "no rule `{}` (`botica rules --all` lists every rule)",
            args.id
        );
    };
    let config = match &project.loaded.source {
        Some(path) => path.display().to_string(),
        None => "none (the defaults apply)".to_owned(),
    };
    let mut out = std::io::stdout().lock();
    let code_dirs = registry::uses_code_dirs(&entry.rule.id).then(|| project.describe_code_dirs());
    write_entry(entry, &config, code_dirs.as_deref(), &mut out)?;
    Ok(())
}

fn or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

fn status_name(status: Status) -> &'static str {
    match status {
        Status::Preview => "preview",
        Status::Stable => "stable",
        Status::Deprecated => "deprecated",
        Status::Removed => "removed",
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

fn write_entry(
    entry: &Entry,
    config: &str,
    code_dirs: Option<&str>,
    out: &mut impl Write,
) -> Result<()> {
    let rule = &entry.rule;
    writeln!(out, "{} ({}): {}\n", rule.id, rule.code, rule.summary)?;
    let state = if entry.active {
        format!("active, {}", entry.reason)
    } else {
        format!("not active, {}", entry.reason)
    };
    writeln!(out, "state:     {state}")?;
    writeln!(out, "severity:  {}", severity_name(entry.severity))?;
    writeln!(
        out,
        "status:    {} (since {})",
        status_name(rule.status),
        rule.since
    )?;
    writeln!(
        out,
        "family:    {}",
        rule.family.as_deref().unwrap_or("none")
    )?;
    if let Some(standard) = &rule.standard {
        writeln!(out, "standard:  {standard}")?;
    }
    writeln!(out, "languages: {}", rule.languages().join(", "))?;
    writeln!(out, "aliases:   {}", or_none(&rule.aliases))?;
    writeln!(out, "tags:      {}", or_none(&rule.tags))?;
    let options: Vec<String> = entry
        .options
        .iter()
        .map(|(name, value)| format!("{name} = {value}"))
        .collect();
    writeln!(out, "options:   {}", or_none(&options))?;
    if let Some(code_dirs) = code_dirs {
        writeln!(out, "code-dirs: {code_dirs}")?;
    }
    writeln!(out, "config:    {config}")?;
    writeln!(
        out,
        "ignores:{}",
        if entry.ignore_patterns.is_empty() {
            " none"
        } else {
            ""
        }
    )?;
    for pattern in &entry.ignore_patterns {
        let origin = match pattern.origin {
            IgnoreOrigin::Rule => "rule",
            IgnoreOrigin::Config => "config",
        };
        writeln!(out, "  {}  ({origin})", pattern.glob)?;
    }
    writeln!(out, "\n{}", rule.readme.trim_end())?;
    Ok(())
}
