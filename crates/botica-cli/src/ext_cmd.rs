//! `botica ext`: what the extension rules of a project need before `botica lint` runs them.

use std::io::Write;
use std::process::{Command, ExitCode, Stdio};

use anyhow::Result;
use botica_catalog::{Kind, Runtime};
use clap::{Args, Subcommand};

use crate::extension_pool::{sdk_of, Launcher};
use crate::project::{self, ProjectArgs};

#[derive(Args)]
pub struct ExtArgs {
    #[command(subcommand)]
    command: ExtCommand,
}

#[derive(Subcommand)]
enum ExtCommand {
    /// Create or update the environment of the script of each active extension rule of the
    /// runtime `python`, with `uv sync --script` and the Python of the project; a script that
    /// declares `botica` with no version also starts once, to install the version of botica.
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

/// Runs `uv sync --script` on the script of a Python rule, with the Python of the project. A
/// script that gets its `botica` from botica (`uv run --with`) then starts once, as `botica lint`
/// starts it, with its stdin closed: `uv sync` does not take `--with`, and that run installs the
/// version of botica in the cache of `uv`.
fn prepare(
    launcher: &Launcher,
    rule: &botica_catalog::Rule,
    implementation: &botica_catalog::Implementation,
    root: &std::path::Path,
) -> Result<(), String> {
    let script = launcher
        .script(rule, implementation)
        .ok_or_else(|| "no script".to_owned())?;
    let mut sync = vec!["uv".to_owned(), "sync".to_owned()];
    sync.extend(launcher.python()?.iter().cloned());
    sync.extend(["--script".to_owned(), script.to_string_lossy().into_owned()]);
    run_quietly(&sync, root)?;
    if sdk_of(&script).is_some() {
        run_quietly(&launcher.command(rule, implementation)?, root)?;
    }
    Ok(())
}

/// Runs `argv` in `root` with no stdin and its stdout left out.
fn run_quietly(argv: &[String], root: &std::path::Path) -> Result<(), String> {
    let status = Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("cannot run `{}`: {error}", argv[0]))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("`{}` exited with {status}", argv[..2].join(" ")))
    }
}
