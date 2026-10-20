//! `settings-env-schema`: the variables that the varlock schema (`.env.schema`) declares and the
//! ones that the settings classes of the project read are the same. Reading the schema and
//! comparing are the same for every language; a reader of a language finds the settings classes
//! of its files (Python and pydantic-settings for now).

mod python;
mod schema;
mod settings;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use settings::Class;

pub const SETTINGS_ENV_SCHEMA: &str = "settings-env-schema";

/// The schema file, at the root of the project.
pub const SCHEMA: &str = ".env.schema";

/// A file of the project and its text.
pub struct Source<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// A language that has a reader of settings classes.
struct Reader {
    extensions: &'static [&'static str],
    /// A file that names none of these, nor a class found elsewhere, holds no settings class.
    root: &'static str,
    classes: fn(&str, &str) -> Vec<Class>,
}

const READERS: [Reader; 1] = [Reader {
    extensions: &["py", "pyi"],
    root: python::ROOT,
    classes: python::classes,
}];

fn reader_of(path: &str) -> Option<&'static Reader> {
    let extension = path.rsplit_once('.')?.1;
    READERS
        .iter()
        .find(|reader| reader.extensions.contains(&extension))
}

/// Whether a reader of settings classes reads `path`, so a caller only loads those files.
pub fn reads(path: &str) -> bool {
    reader_of(path).is_some()
}

/// The classes of the sources that may hold settings classes: the files that name the root class
/// of their language, then those that name a class already found, until no file is left.
fn classes_of(sources: &[Source<'_>]) -> Vec<Class> {
    let mut pending: Vec<(&Source<'_>, &Reader)> = sources
        .iter()
        .filter_map(|source| Some((source, reader_of(source.path)?)))
        .collect();
    let mut classes: Vec<Class> = Vec::new();
    loop {
        let (now, later): (Vec<_>, Vec<_>) = pending.into_iter().partition(|(source, reader)| {
            source.text.contains(reader.root)
                || classes
                    .iter()
                    .any(|class| source.text.contains(&class.name))
        });
        if now.is_empty() {
            return classes;
        }
        for (source, reader) in now {
            classes.extend((reader.classes)(source.path, source.text));
        }
        pending = later;
    }
}

/// A finding of the rule over a byte range of `text`.
fn finding(
    path: &str,
    text: &str,
    range: (usize, usize),
    severity: Severity,
    message: String,
) -> Diagnostic {
    let document = Document::new(path, text);
    let (line, column) = document.position(range.0);
    let (end_line, end_column) = document.position(range.1);
    Diagnostic {
        path: path.to_owned(),
        rule: SETTINGS_ENV_SCHEMA.to_owned(),
        severity,
        start: range.0,
        end: range.1,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        message,
    }
}

/// The fields of the settings classes whose variable the schema lacks, and the variables of the
/// schema that no settings class reads, unless they carry `@auditIgnore`. The second kind needs a
/// settings class in the project. Paths are those of the sources, and `SCHEMA` for the schema.
pub fn check(schema: &str, sources: &[Source<'_>], severity: Severity) -> Vec<Diagnostic> {
    let variables = schema::variables(schema);
    let classes = classes_of(sources);
    let (declarations, has_settings) = settings::declarations(&classes);
    let mut found = Vec::new();
    for declaration in &declarations {
        let declared = variables
            .iter()
            .any(|variable| declaration.reads(&variable.name));
        let Some(source) = sources.iter().find(|s| s.path == declaration.path) else {
            continue;
        };
        if declared || !declaration.determined() {
            continue;
        }
        let (field, class, expected) = (
            &declaration.field.name,
            declaration.class,
            &declaration.expected,
        );
        found.push(finding(
            source.path,
            source.text,
            (declaration.field.start, declaration.field.end),
            severity,
            format!(
                "the field `{field}` of `{class}` reads \
                 `{expected}`, which {SCHEMA} does not declare: add `{expected}` to {SCHEMA}"
            ),
        ));
    }
    if has_settings {
        for variable in variables.iter().filter(|variable| !variable.audit_ignore) {
            if declarations.iter().any(|d| d.reads(&variable.name)) {
                continue;
            }
            let name = &variable.name;
            found.push(finding(
                SCHEMA,
                schema,
                (variable.start, variable.end),
                severity,
                format!(
                    "{SCHEMA} declares `{name}`, and no settings \
                     class of the project reads it: add a field that reads it, remove it, or mark \
                     it `# @auditIgnore` when another program reads it"
                ),
            ));
        }
    }
    found.sort_by(|a, b| (&a.path, a.start).cmp(&(&b.path, b.start)));
    found
}
