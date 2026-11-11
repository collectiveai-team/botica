//! `botica history report` (ADR 0019 decision 10): the reader of the history of findings.

mod ancestry;
mod lines;
mod order;
mod pairs;
mod quiet;
mod read;
mod render;
mod report;

use crate::output::print;
use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use quiet::Noise;
use render::Format;

#[derive(Args)]
pub struct HistoryArgs {
    #[command(subcommand)]
    command: HistoryCommand,
}

#[derive(Subcommand)]
enum HistoryCommand {
    /// Read the history of findings: the rules with no findings, the rules with many
    /// suppressions, and the findings that reached CI or were caught before the commit.
    Report(ReportArgs),
}

#[derive(Args)]
struct ReportArgs {
    /// How many days back the report reads.
    #[arg(long, default_value_t = 28)]
    days: u64,
    /// The format of the report.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// A folder with one folder for each run of CI. A run folder holds `source.json`, which says
    /// whether the run is trusted, and the files `runs.jsonl` and `findings.jsonl`, either in it
    /// or in a folder of it for each artifact. Repeat the option for more folders.
    #[arg(long)]
    from: Vec<PathBuf>,
    /// A rule has many suppressions when at least this many of its findings are suppressed.
    #[arg(long, default_value_t = 5)]
    min_suppressed: u64,
    /// A rule has many suppressions when at least this percent of its findings are suppressed.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(u8).range(0..=100))]
    min_share: u8,
}

pub fn run(args: &HistoryArgs) -> Result<ExitCode> {
    match &args.command {
        HistoryCommand::Report(report) => report_run(report),
    }
}

fn report_run(args: &ReportArgs) -> Result<ExitCode> {
    if args.days == 0 {
        bail!("--days must be at least 1");
    }
    let start = std::env::current_dir().context("cannot read the working directory")?;
    let root = botica_config::discover(&start)?.root;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let history = read::load(&root, &args.from, report::since(now, args.days))?;
    let noise = Noise {
        min_suppressed: args.min_suppressed,
        min_share: f64::from(args.min_share) / 100.0,
    };
    let git = ancestry::Git::at(root);
    let report = report::build(&history, now, args.days, noise, &git);
    let text = render::render(&report, args.format);
    print_report(&text);
    Ok(ExitCode::SUCCESS)
}

/// Inside GitHub Actions a line that holds `::` or `##[` could be read as a workflow command, so
/// the report is printed between `::stop-commands::<token>` and `::<token>::` (ADR 0019 decision
/// 9.4).
fn print_report(text: &str) {
    let in_actions = std::env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true");
    if !in_actions {
        print!("{text}");
        return;
    }
    let token = random_token();
    print!("::stop-commands::{token}\n{text}::{token}::\n");
}

/// A token for the stop line: blake3 of the clock, the process id and the address of a stack
/// variable. The lines of the history are fixed before the process starts, so they cannot predict it.
fn random_token() -> String {
    let marker = 0u8;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let mut hasher = blake3::Hasher::new();
    hasher.update(&nanos.to_le_bytes());
    hasher.update(&std::process::id().to_le_bytes());
    hasher.update(&(std::ptr::addr_of!(marker) as usize).to_le_bytes());
    hasher.finalize().to_hex()[..32].to_owned()
}
