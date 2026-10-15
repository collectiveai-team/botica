mod catalog_json;
mod explain_cmd;
mod extension;
mod lint_cmd;
mod project;
mod rule_page;
mod rule_view;
mod rules;
mod rules_cmd;
mod rules_export;
mod timings;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "botica", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Lint(lint_cmd::LintArgs),
    /// Show which rules are active and why.
    Rules(rules_cmd::RulesArgs),
    /// Say what a rule is, how it is configured here and why it is or is not active.
    Explain(explain_cmd::ExplainArgs),
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Lint(args) => lint_cmd::run(args),
        Command::Rules(args) => rules_cmd::run(&args).map(|()| ExitCode::SUCCESS),
        Command::Explain(args) => explain_cmd::run(&args).map(|()| ExitCode::SUCCESS),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("botica: {error:#}");
            ExitCode::from(2)
        }
    }
}
