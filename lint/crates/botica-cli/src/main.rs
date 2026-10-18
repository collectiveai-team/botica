mod agents_md;
mod catalog_json;
mod explain_cmd;
mod extension;
mod extension_pool;
mod lint_cmd;
mod project;
mod recipes_check;
mod recipes_cmd;
mod recipes_collection;
mod refs_check;
mod refs_cmd;
mod refs_collection;
mod refs_edit;
mod refs_export;
mod refs_pin;
mod refs_remote;
mod refs_standards;
mod refs_update;
mod rubric;
mod rule_page;
mod rule_view;
mod rules;
mod rules_cmd;
mod rules_conform;
mod rules_export;
mod rules_new;
mod rules_new_extension;
mod rules_new_target;
mod rules_test;
mod server_cmd;
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
    /// Read the code that the rules and the standards point at, in `references/`.
    Refs(refs_cmd::RefsArgs),
    /// Read the recipes, the lists of rules and exports that a repository of some kind has, and
    /// check a project against one.
    Recipe(recipes_cmd::RecipesArgs),
    /// Serve the findings of the rules to an editor, over the Language Server Protocol on stdio.
    Server(server_cmd::ServerArgs),
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Command::Lint(args) => lint_cmd::run(args),
        Command::Rules(args) => rules_cmd::run(&args),
        Command::Explain(args) => explain_cmd::run(&args).map(|()| ExitCode::SUCCESS),
        Command::Refs(args) => refs_cmd::run(&args),
        Command::Recipe(args) => recipes_cmd::run(&args),
        Command::Server(args) => server_cmd::run(args).map(|()| ExitCode::SUCCESS),
    };
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("botica: {error:#}");
            ExitCode::from(2)
        }
    }
}
