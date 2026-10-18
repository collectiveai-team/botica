//! `botica recipe check`: how the project differs from a recipe.

use std::process::ExitCode;

use anyhow::Result;
use botica_catalog::Kind;

use crate::project::ProjectArgs;
use crate::recipes_collection::Collection;
use crate::rules_export;

/// One line for each rule and export of the recipe, then a summary. The command fails when a
/// rule is not active or an export is not current.
pub fn run(collection: &Collection, name: &str, project: &ProjectArgs) -> Result<ExitCode> {
    let resolved = collection
        .loaded()?
        .resolve(name)
        .map_err(anyhow::Error::msg)?;
    let mut lines = Vec::new();
    for item in &resolved.rules {
        lines.push(line(
            "rule",
            &item.id,
            rule_problem(collection, &item.id, false),
        ));
    }
    for item in &resolved.judgment {
        lines.push(line(
            "judgment rule",
            &item.id,
            rule_problem(collection, &item.id, true),
        ));
    }
    for item in &resolved.exports {
        let problem = rules_export::check_export(&item.id, project)
            .err()
            .map(|error| error.to_string());
        lines.push(line("export", &item.id, problem));
    }
    let differ = lines
        .iter()
        .filter(|line| line.contains(": differs: "))
        .count();
    for line in &lines {
        println!("{line}");
    }
    let verb = if differ == 1 { "differs" } else { "differ" };
    println!("{name}: {} checked, {differ} {verb}", lines.len());
    Ok(if differ == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn line(kind: &str, id: &str, problem: Option<String>) -> String {
    match problem {
        None => format!("{kind} {id}: ok"),
        Some(problem) => format!("{kind} {id}: differs: {problem}"),
    }
}

/// Why the project does not run the rule `key` as the recipe lists it, if it does not.
fn rule_problem(collection: &Collection, key: &str, judgment: bool) -> Option<String> {
    let Some(entry) = collection.project.resolution.entry(key) else {
        return Some("not in the catalog".to_owned());
    };
    let is_judgment = entry
        .rule
        .implementations
        .iter()
        .any(|implementation| implementation.kind == Kind::Judgment);
    if judgment && !is_judgment {
        return Some("is not a judgment rule".to_owned());
    }
    if !judgment && is_judgment {
        return Some("is a judgment rule: list it under `judgment`".to_owned());
    }
    (!entry.active).then(|| entry.reason.to_string())
}
