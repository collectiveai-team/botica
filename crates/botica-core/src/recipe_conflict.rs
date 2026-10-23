//! `recipe-conflict`: two recipes that a project uses ask for one package of a tool with
//! different requirements, which `botica recipe install` refuses (`Recipes::tool_conflicts`).

use std::ops::Range;

use botica_catalog::ToolConflict;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;

pub const RECIPE_CONFLICT: &str = "recipe-conflict";

/// A recipe of `recipes/`: its id, its path from the root of the project and its text.
pub struct RecipeFile<'a> {
    pub id: &'a str,
    pub path: &'a str,
    pub text: &'a str,
}

/// One finding for each conflict, on the requirement in the file of the later recipe, or of the
/// earlier one when the later one is built in, and about the whole file `fallback` when both are
/// built in.
pub fn check(
    conflicts: &[ToolConflict],
    files: &[RecipeFile<'_>],
    fallback: &str,
    severity: Severity,
) -> Vec<Diagnostic> {
    conflicts
        .iter()
        .map(|conflict| {
            let message = message(conflict);
            let place = [&conflict.then, &conflict.first]
                .into_iter()
                .find_map(|item| {
                    let file = files.iter().find(|file| file.id == item.from)?;
                    Some((file, requirement(file.text, &item.id)))
                });
            match place {
                Some((file, Some(span))) => on_a_line(file, span, severity, message),
                Some((file, None)) => {
                    Diagnostic::file_level(file.path, RECIPE_CONFLICT, severity, message)
                }
                None => Diagnostic::file_level(fallback, RECIPE_CONFLICT, severity, message),
            }
        })
        .collect()
}

/// A finding about the whole file `path` when botica cannot resolve the recipes of the project:
/// `recipes/` does not load, or the configuration installs a recipe that does not exist.
pub fn unresolved(path: &str, problem: &str, severity: Severity) -> Diagnostic {
    Diagnostic::file_level(
        path,
        RECIPE_CONFLICT,
        severity,
        format!(
            "botica cannot resolve the recipes of the project, so it does not look for conflicts \
             between them: {problem}"
        ),
    )
}

/// What the finding about `conflict` says, which `botica recipe check` prints too.
pub fn message(conflict: &ToolConflict) -> String {
    format!(
        "The recipes `{}` and `{}` ask for the package `{}` as `{}` and as `{}`: a package has one \
         requirement across the recipes, and `botica recipe install` stops on them",
        conflict.then.from, conflict.first.from, conflict.package, conflict.then.id, conflict.first.id
    )
}

/// Where `requirement` is written in the text of a recipe, with its quotes.
fn requirement(text: &str, requirement: &str) -> Option<Range<usize>> {
    ["\"", "'"].iter().find_map(|quote| {
        let quoted = format!("{quote}{requirement}{quote}");
        let at = text.find(&quoted)?;
        Some(at..at + quoted.len())
    })
}

fn on_a_line(
    file: &RecipeFile<'_>,
    span: Range<usize>,
    severity: Severity,
    message: String,
) -> Diagnostic {
    let document = Document::new(file.path, file.text);
    let (line, column) = document.position(span.start);
    let (end_line, end_column) = document.position(span.end);
    Diagnostic {
        path: file.path.to_owned(),
        rule: RECIPE_CONFLICT.to_owned(),
        severity,
        start: span.start,
        end: span.end,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        measure: None,
        message,
    }
}
