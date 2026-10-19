//! `botica rules eval`: what one deterministic rule finds in real code, whatever the
//! configuration selects, for a review of its false positives.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::Args;

use crate::lint_cmd::{self, LintArgs};
use crate::project::{self, ProjectArgs};

#[derive(Args)]
pub struct EvalArgs {
    /// The rule: its slug, its code or an alias.
    rule: String,
    /// Directories to walk, or files; the current directory when none is given.
    paths: Vec<PathBuf>,
    #[command(flatten)]
    project: ProjectArgs,
}

pub fn run(args: &EvalArgs) -> Result<ExitCode> {
    let loaded = project::load(&args.project)?;
    let Some(entry) = loaded.resolution.entry(&args.rule) else {
        bail!(
            "no rule `{}` (`botica rules --all` lists every rule)",
            args.rule
        );
    };
    let rule = &entry.rule;
    if rule.is_judgment() {
        bail!(
            "a judge evaluates `{}`: `botica rules test --judge abide` calibrates it on its fixtures",
            rule.id
        );
    }
    if rule.is_external_only() {
        bail!("another tool enforces `{}`: run that tool", rule.id);
    }
    let lint = LintArgs::for_eval(args.paths.clone(), args.project.only(&rule.id));
    lint_cmd::run(lint)
}
