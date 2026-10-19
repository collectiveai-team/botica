//! `botica rules conform`: runs each rule of a catalog over its fixtures, with and without each
//! optimisation, through this same binary.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use botica_catalog::CATALOG_FILE;
use botica_conformance::{Outcome, Runner};
use clap::Args;

/// Where the projects of the fixtures go, from where botica runs.
const SCRATCH_DIR: &str = ".botica/conformance";

#[derive(Args)]
pub struct ConformArgs {
    /// The folder of the catalog: `catalog/rules` for the built-in one, or `.botica/rules/<name>` for
    /// a catalog of a project (the one with a `catalog.toml`).
    catalog: PathBuf,
}

pub fn run(args: &ConformArgs) -> Result<ExitCode> {
    let binary = std::env::current_exe().context("cannot find the botica binary")?;
    let runner = Runner::new(binary, SCRATCH_DIR);
    let outcomes = if args.catalog.join(CATALOG_FILE).is_file() {
        runner.check_project_catalog(&args.catalog)?
    } else {
        runner.check_catalog(&args.catalog)?
    };
    for outcome in &outcomes {
        report(outcome);
    }
    let conforming = outcomes.iter().filter(|outcome| outcome.passed()).count();
    println!("{conforming} of {} rules conform", outcomes.len());
    Ok(if conforming == outcomes.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn report(outcome: &Outcome) {
    if outcome.passed() {
        println!("ok   {}", outcome.rule());
        return;
    }
    println!("FAIL {}", outcome.rule());
    for failure in outcome.failures() {
        println!("  {failure}");
    }
}
