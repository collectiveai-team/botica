use std::io::Write;

use anyhow::{bail, Result};
use botica_catalog::{Catalog, Kind, Severity, Status};
use botica_config::{Entry, IgnoreOrigin};
use botica_core::registry;
use clap::Args;

use crate::extension_pool::Launcher;
use crate::project::{self, ProjectArgs};
use crate::rule_view::Programs;

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
    let since = Catalog::embedded()?.lifecycle(&entry.rule).since;
    let shown = Shown {
        since: since.as_deref(),
        config: &config,
        code_dirs: code_dirs.as_deref(),
        programs: Programs {
            launcher: Launcher::of(&project),
            root: &project.loaded.root,
        },
    };
    write_entry(entry, &shown, &mut out)?;
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

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Native => "native",
        Kind::AstGrep => "ast-grep",
        Kind::Extension => "extension",
        Kind::Judgment => "judgment",
        Kind::External => "external",
    }
}

/// What the configuration resolved for one rule, and how the rule is built.
struct Shown<'a> {
    since: Option<&'a str>,
    config: &'a str,
    code_dirs: Option<&'a str>,
    programs: Programs<'a>,
}

fn write_entry(entry: &Entry, shown: &Shown<'_>, out: &mut impl Write) -> Result<()> {
    let (since, config, code_dirs, programs) =
        (shown.since, shown.config, shown.code_dirs, &shown.programs);
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
        "status:    {} ({})",
        status_name(rule.status),
        since.map_or_else(|| "not released yet".to_owned(), |v| format!("since {v}"))
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
    for implementation in &rule.implementations {
        let mut line = format!(
            "kind:      {} ({})",
            kind_name(implementation.kind),
            implementation.languages.join(", ")
        );
        if let Some(argv) = programs.runs(rule, implementation) {
            line.push_str(&format!(", runs {}", argv.join(" ")));
        }
        if let Some(tool) = &implementation.tool {
            line.push_str(&format!(", tool {tool}"));
        }
        writeln!(out, "{line}")?;
    }
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
