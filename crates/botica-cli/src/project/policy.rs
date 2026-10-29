use std::collections::BTreeMap;
use std::sync::Arc;

use botica_catalog::Scope;
use botica_config::Resolution;
use botica_core::report::RuleDoc;
use botica_core::suppress::{Policy, INVALID, UNUSED};
use botica_core::{postprocess, registry, Diagnostic, Severity};

use super::verdicts;

/// What the suppression comments may name, and whether the rules about them report.
pub fn suppression_policy(resolution: &Arc<Resolution>) -> Policy {
    let mut policy = Policy::new();
    for code in resolution.project_codes() {
        policy = policy.with_code_prefix(code);
    }
    for entry in resolution.entries() {
        let spellings: Vec<&str> = std::iter::once(entry.rule.code.as_str())
            .chain(entry.rule.aliases.iter().map(String::as_str))
            .chain(resolution.retired_names_of(&entry.rule.id))
            .collect();
        policy = if entry.rule.is_judgment() {
            policy.with_judgment_rule(&entry.rule.id, &spellings)
        } else if entry.rule.has_extension() {
            policy.with_extension_rule(&entry.rule.id, &spellings, entry.active)
        } else if registry::silenced_on_its_line(&entry.rule.id) {
            policy.with_line_project_rule(&entry.rule.id, &spellings)
        } else if matches!(entry.rule.scope, Some(Scope::Project | Scope::Commit)) {
            policy.with_project_rule(&entry.rule.id, &spellings)
        } else {
            policy.with_rule(&entry.rule.id, &spellings)
        };
    }
    let external = Arc::clone(resolution);
    policy = policy.with_external(move |id| external.is_external(id));
    // With `--scope project`, `unused-suppression` is active for the pass after the rules of
    // project scope alone: the walk would report again what `--scope file` reports.
    if let Some(level) =
        reporting(resolution, UNUSED).filter(|_| resolution.scope() != Some(Scope::Project))
    {
        policy = policy.report_unused(level);
    }
    if let Some(level) = reporting(resolution, INVALID) {
        policy = policy.report_invalid(level);
    }
    policy
}

/// The policy of the pass after the rules of project scope, which reports the comments that name
/// one whose findings sit on a line and silence nothing, whatever the scope of the run.
pub fn project_pass_policy(resolution: &Arc<Resolution>) -> Policy {
    let policy = suppression_policy(resolution);
    match reporting(resolution, UNUSED) {
        Some(level) => policy.report_unused(level),
        None => policy,
    }
}

/// The rules of another scope that the run of `scope` reports too: with `--scope project`,
/// `unused-suppression`, since only the project pass tells whether a comment that names a rule
/// of project scope silences anything.
pub fn reported_too(scope: Option<Scope>) -> &'static [&'static str] {
    if scope == Some(Scope::Project) {
        &[UNUSED]
    } else {
        &[]
    }
}

fn reporting(resolution: &Resolution, slug: &str) -> Option<Severity> {
    resolution
        .entry(slug)
        .filter(|entry| entry.active)
        .map(|entry| Severity::from(entry.severity))
}

/// What the project keeps of findings of judgment rules, grouped by path: those that no
/// `ignore-file` comment of their file silences and that the configuration applies at their path
/// and in their language, with the severity it names there. `read` gives the text of a file.
pub fn keep_judged(
    resolution: &Arc<Resolution>,
    prefix: Option<&str>,
    by_path: BTreeMap<String, Vec<Diagnostic>>,
    read: impl Fn(&str) -> Option<String>,
) -> Vec<Diagnostic> {
    let policy = suppression_policy(resolution);
    let mut found = Vec::new();
    for (path, mut findings) in by_path {
        if let Some(text) = read(&path) {
            policy.silence(&path, &text, &mut findings);
        }
        found.extend(findings);
    }
    postprocess::apply(found, verdicts(resolution, prefix))
}

/// The catalog's rules as the output formats describe them.
pub fn rule_docs(resolution: &Resolution) -> Vec<RuleDoc> {
    resolution
        .entries()
        .iter()
        .map(|entry| RuleDoc {
            id: entry.rule.id.clone(),
            code: entry.rule.code.clone(),
            standard: entry.rule.standard.clone(),
            summary: entry.rule.summary.clone(),
            severity: Severity::from(entry.severity),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use botica_astgrep::AstGrepSet;
    use botica_catalog::{read_files, Catalog, SourceFile};
    use botica_config::Config;
    use botica_core::Engine;

    const RULE: &str = "id = \"test-foo\"\ncode = \"BTTEST901\"\nsummary = \"s\"\n\
        status = \"stable\"\ndefault-enabled = true\ndefault-severity = \"warning\"\n\n\
        [[implementations]]\nkind = \"judgment\"\nlanguages = [\"python\"]\nwhen = \"edit\"\n\
        why-not-deterministic = \"w\"\n\n\
        [implementations.question]\ntype = \"boolean\"\ninstructions = \"q\"\n";

    const README: &str = "## Directive\n\nd\n\n## Why\n\nw\n\n## Flag\n\nf\n\n\
        ## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
        ## Suppression\n\ns\n";

    /// The catalog of this checkout with one judgment rule, `test-foo`.
    fn lint(text: &str) -> Vec<String> {
        let rules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../catalog/rules");
        let mut files = read_files(&rules).expect("the catalog is read");
        for (name, text) in [("rule.toml", RULE), ("README.md", README)] {
            files.push(SourceFile {
                path: format!("testing/test-foo/{name}"),
                text: text.to_owned(),
            });
        }
        let catalog = Catalog::from_files(&files).unwrap_or_else(|error| panic!("{error}"));
        let config = Config::from_botica_toml("").expect("an empty configuration");
        let resolution = Arc::new(Resolution::new(&config, &catalog).expect("it resolves"));
        Engine::new(Vec::new(), AstGrepSet::default())
            .with_suppressions(suppression_policy(&resolution))
            .lint_source("a.py", text)
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect()
    }

    #[test]
    fn a_judgment_rule_of_the_catalog_is_suppressed_by_file_with_a_reason_only() {
        assert!(lint("# botica: ignore-file[test-foo] -- a reason\n").is_empty());
        assert_eq!(
            lint("x = 1  # botica: ignore[BTTEST901]\n"),
            [
                "`BTTEST901` is a judgment rule: suppress it for the whole file with \
              `botica: ignore-file[test-foo] -- <reason>`"
            ]
        );
    }

    #[test]
    fn a_suppression_may_name_a_retired_code_of_a_rule() {
        let found = lint("x = 1  # noqa: BTARCH005\n");

        assert!(
            found
                .iter()
                .all(|message| !message.starts_with("Unknown rule")),
            "{found:?}"
        );
    }

    #[test]
    fn a_suppression_that_names_no_rule_is_still_reported() {
        let found = lint("x = 1  # noqa: BTARCH099\n");

        assert!(
            found.contains(&"Unknown rule `BTARCH099` in a suppression".to_owned()),
            "{found:?}"
        );
    }
}
