//! `botica ext`: what the extension rules of a project need before `botica lint` runs them.

use std::io::Write;
use std::process::{Command, ExitCode, Stdio};

use anyhow::Result;
use botica_catalog::{Kind, Runtime};
use clap::{Args, Subcommand};

use crate::extension_pool::Launcher;
use crate::project::{self, ProjectArgs};

#[derive(Args)]
pub struct ExtArgs {
    #[command(subcommand)]
    command: ExtCommand,
}

#[derive(Subcommand)]
enum ExtCommand {
    /// Create or update the environment of the script of each active extension rule of the
    /// runtime `python`, with `uv sync --script` and the Python of the project.
    Sync(SyncArgs),
}

#[derive(Args)]
struct SyncArgs {
    #[command(flatten)]
    project: ProjectArgs,
}

pub fn run(args: &ExtArgs) -> Result<ExitCode> {
    let ExtCommand::Sync(sync) = &args.command;
    let project = project::load(&sync.project)?;
    let launcher = Launcher::of(&project);
    let mut out = std::io::stdout().lock();
    let mut failed = false;
    for entry in project.resolution.entries().iter().filter(|e| e.active) {
        let rule = &entry.rule;
        for implementation in &rule.implementations {
            if implementation.kind != Kind::Extension {
                continue;
            }
            if implementation.runtime != Some(Runtime::Python) {
                writeln!(out, "{}: nothing to prepare", rule.id)?;
                continue;
            }
            match prepare(&launcher, rule, implementation, &project.loaded.root) {
                Ok(()) => writeln!(out, "{}: ready", rule.id)?,
                Err(why) => {
                    failed = true;
                    writeln!(out, "{}: failed: {why}", rule.id)?;
                }
            }
        }
    }
    Ok(if failed {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

/// Runs `uv sync --script` on the script of a Python rule, with the Python of the project.
fn prepare(
    launcher: &Launcher,
    rule: &botica_catalog::Rule,
    implementation: &botica_catalog::Implementation,
    root: &std::path::Path,
) -> Result<(), String> {
    let script = launcher
        .script(rule, implementation)
        .ok_or_else(|| "no script".to_owned())?;
    let status = Command::new("uv")
        .arg("sync")
        .args(launcher.python()?)
        .arg("--script")
        .arg(&script)
        .current_dir(root)
        .stdin(Stdio::null())
        .status()
        .map_err(|error| format!("cannot run `uv`: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`uv sync` exited with {status}"))
    }
}
