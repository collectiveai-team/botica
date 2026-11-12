//! `botica review`: the commands that read the `review` table of the configuration (ADR 0018).

mod codeowners;
mod pattern;
mod target;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};

use crate::project::{self, ProjectArgs};

#[derive(Args)]
pub struct ReviewArgs {
    #[command(subcommand)]
    command: ReviewCommand,
}

#[derive(Subcommand)]
enum ReviewCommand {
    /// Write the areas of `[[review.areas]]` as a file that a code host reads, as CODEOWNERS.
    Export(ExportArgs),
}

#[derive(Args)]
struct ExportArgs {
    /// What to write.
    #[arg(long, value_enum)]
    format: Format,
    /// The code host that reads the file. GitLab gets one section for each area, with the
    /// approvals of the area.
    #[arg(long, value_enum, default_value_t = Host::Github)]
    host: Host,
    /// The file to write, or `-` for the standard output. Default: `.github/CODEOWNERS` or
    /// `.gitlab/CODEOWNERS` under the root of the project.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Write nothing, and fail when what the export would write differs from what is there.
    #[arg(long)]
    check: bool,
    #[command(flatten)]
    project: ProjectArgs,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    /// The owners of each path: the paths of each area with the owners of its entry.
    Codeowners,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Host {
    Github,
    Gitlab,
}

impl Host {
    fn name(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
        }
    }
}

pub fn run(args: &ReviewArgs) -> Result<ExitCode> {
    match &args.command {
        ReviewCommand::Export(export) => run_export(export).map(|()| ExitCode::SUCCESS),
    }
}

fn run_export(args: &ExportArgs) -> Result<()> {
    let Format::Codeowners = args.format;
    let project = project::load(&args.project)?;
    let root = &project.loaded.root;
    let target = target::Target::of(args.output.as_deref(), args.host, root, args.check)?;
    target.refuse_shadowed(args.host, root)?;
    let text = codeowners::render(&project.loaded.config.review, args.host)?;
    target.deliver(&text, args.check, args.host)
}
