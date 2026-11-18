//! `botica review`: the commands that read the `review` table of the configuration (ADR 0018).

mod automation;
mod change_context;
mod classification_history;
mod classify;
mod codeowners;
mod consumers;
mod evaluate;
mod evidence;
mod feedback;
mod github;
mod http;
mod jev;
mod pattern;
mod runner;
mod target;
mod tier;

use crate::output::println;
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
    /// Read the linked issue bodies and PR metadata from GitHub for pinned inputs.
    Context(github::ContextArgs),
    /// Record human review observations for a pinned commit, without approving or merging.
    Feedback(feedback::FeedbackArgs),
    /// Measure recorded classifier observations against a pinned labeled set.
    Evaluate(evaluate::EvaluateArgs),
    /// Authorize from fresh evidence and a measured destination policy, without merging.
    Authorize(automation::AutomationArgs),
    /// Select review roles using pinned destination policy.
    Plan(consumers::ConsumerArgs),
    /// Validate trusted role definitions and local skill availability.
    Check(consumers::ConsumerArgs),
    /// Validate current role results and recompute their verdicts.
    Validate(evidence::EvidenceArgs),
    /// Execute planned OpenAI role reviews without tools, then refute.
    Run(runner::RunArgs),
    /// Recommend from current evidence in shadow mode, without merging.
    Classify(classify::ClassifyArgs),
    /// Assess branch risk and integration risk from pinned Git inputs.
    Tier(tier::TierArgs),
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
    fn name(self) -> String {
        self.to_possible_value()
            .map(|value| value.get_name().to_owned())
            .unwrap_or_default()
    }
}

pub fn run(args: &ReviewArgs) -> Result<ExitCode> {
    match &args.command {
        ReviewCommand::Context(args) => {
            println!("{}", serde_json::to_string_pretty(&github::context(args)?)?);
            Ok(ExitCode::SUCCESS)
        }
        ReviewCommand::Plan(plan) => consumers::run(plan, false),
        ReviewCommand::Check(check) => consumers::run(check, true),
        ReviewCommand::Validate(args) => evidence::run(args),
        ReviewCommand::Run(args) => runner::run(args),
        ReviewCommand::Classify(args) => classify::run(args),
        ReviewCommand::Feedback(args) => feedback::run(args),
        ReviewCommand::Evaluate(args) => evaluate::run(args),
        ReviewCommand::Authorize(args) => automation::run(args),
        ReviewCommand::Tier(tier) => tier::run(tier),
        ReviewCommand::Export(export) => run_export(export).map(|()| ExitCode::SUCCESS),
    }
}

pub(crate) fn reviewed(
    diff: String,
    head: String,
    ci: bool,
    context: Option<PathBuf>,
    directory: &std::path::Path,
    recording: bool,
) -> Result<ExitCode> {
    let args = consumers::ConsumerArgs::evidence(diff, head, ci, context);
    let (_, result) = evidence::validate(&args, directory, recording)?;
    evidence::print(&result)
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
