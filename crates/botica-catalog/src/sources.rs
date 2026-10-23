//! The files an implementation names: the YAML of an ast-grep rule and the fragment of
//! configuration of an external one.

use crate::config_format::Format;
use crate::issue::{issue, CatalogIssue};
use crate::load::SourceFile;
use crate::model::{Kind, Rule};

/// What is wrong with the top-level `id:` of an ast-grep YAML, if anything: diagnostics carry
/// that id, so it has to be the slug the catalog, the selection and the suppressions know.
fn id_problem(yaml: &str, slug: &str) -> Option<String> {
    let found: Vec<&str> = yaml
        .lines()
        .filter_map(|line| line.strip_prefix("id:"))
        .map(|id| id.trim().trim_matches(['"', '\'']))
        .collect();
    (found != [slug]).then(|| {
        if found.is_empty() {
            format!("the YAML must declare `id: {slug}`")
        } else {
            format!(
                "the YAML must declare only `id: {slug}`, found `{}`",
                found.join("`, `")
            )
        }
    })
}

/// Gives each ast-grep implementation the text of the file it names, and rejects the files that
/// are named by nobody, the ones that are missing and the implementations that name none.
pub(crate) fn attach_sources(
    base: &str,
    rule: &mut Rule,
    sources: &[&SourceFile],
    issues: &mut Vec<CatalogIssue>,
) {
    let mut named = Vec::new();
    for implementation in &mut rule.implementations {
        if implementation.kind != Kind::AstGrep {
            continue;
        }
        let Some(name) = implementation.file.clone() else {
            issues.push(issue(
                format!("{base}/rule.toml"),
                "an ast-grep implementation must name its YAML with `file`",
            ));
            continue;
        };
        let path = format!("{base}/{name}");
        match sources.iter().find(|source| source.path == path) {
            Some(source) => {
                if let Some(problem) = id_problem(&source.text, &rule.id) {
                    issues.push(issue(path, problem));
                }
                implementation.source = Some(source.text.clone());
            }
            None => issues.push(issue(
                path,
                format!(
                    "`{name}` is named by an implementation of `{}` and is missing",
                    rule.id
                ),
            )),
        }
        named.push(name);
    }
    reject_unnamed(base, sources, &named, "file", issues);
}

/// Whether a file of the folder of a rule is a script of an extension: Python or Node.
pub(crate) fn is_script(name: &str) -> bool {
    [".py", ".js", ".mjs"]
        .iter()
        .any(|extension| name.ends_with(extension))
}

/// Gives each extension that names a script the text of it, and rejects the ones whose script is
/// missing. The other scripts of the folder are the modules that the named ones import.
pub(crate) fn attach_scripts(
    base: &str,
    rule: &mut Rule,
    scripts: &[&SourceFile],
    issues: &mut Vec<CatalogIssue>,
) {
    for implementation in &mut rule.implementations {
        if implementation.kind != Kind::Extension {
            continue;
        }
        let Some(name) = implementation
            .file
            .clone()
            .filter(|name| !name.contains('/'))
        else {
            continue;
        };
        let path = format!("{base}/{name}");
        match scripts.iter().find(|script| script.path == path) {
            Some(script) => implementation.source = Some(script.text.clone()),
            None => issues.push(issue(
                path,
                format!(
                    "`{name}` is named by an implementation of `{}` and is missing",
                    rule.id
                ),
            )),
        }
    }
}

/// Gives each external implementation the text of the fragment that `config` names, and rejects
/// the fragments that are named by nobody, the ones that are missing and those that do not parse
/// in the format of their file.
pub(crate) fn attach_configs(
    base: &str,
    rule: &mut Rule,
    configs: &[&SourceFile],
    issues: &mut Vec<CatalogIssue>,
) {
    let mut named = Vec::new();
    for implementation in &mut rule.implementations {
        let Some(name) = implementation.config.clone() else {
            continue;
        };
        let path = format!("{base}/{name}");
        match configs.iter().find(|config| config.path == path) {
            Some(config) => {
                let format = implementation.target().and_then(Format::of);
                if let Some(Err(error)) = format.map(|format| format.parse(&config.text)) {
                    issues.push(issue(path, error));
                }
                implementation.source = Some(config.text.clone());
            }
            None => issues.push(issue(
                path,
                format!(
                    "`{name}` is named by an implementation of `{}` and is missing",
                    rule.id
                ),
            )),
        }
        named.push(name);
    }
    reject_unnamed(base, configs, &named, "config", issues);
}

fn reject_unnamed(
    base: &str,
    files: &[&SourceFile],
    named: &[String],
    field: &str,
    issues: &mut Vec<CatalogIssue>,
) {
    for file in files {
        let used = named
            .iter()
            .any(|name| file.path == format!("{base}/{name}"));
        if !used {
            issues.push(issue(
                file.path.as_str(),
                format!("no implementation names this file with `{field}`"),
            ));
        }
    }
}
