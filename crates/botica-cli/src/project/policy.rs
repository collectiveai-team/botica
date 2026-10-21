use std::collections::BTreeMap;
use std::sync::Arc;

use botica_catalog::Scope;
use botica_config::Resolution;
use botica_core::report::RuleDoc;
use botica_core::suppress::{Policy, INVALID, UNUSED};
use botica_core::{postprocess, Diagnostic, Severity};

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
            .collect();
        policy = if entry.rule.is_judgment() {
            policy.with_judgment_rule(&entry.rule.id, &spellings)
        } else if entry.rule.has_extension() {
            policy.with_extension_rule(&entry.rule.id, &spellings, entry.active)
        } else if matches!(entry.rule.scope, Some(Scope::Project | Scope::Commit)) {
            policy.with_project_rule(&entry.rule.id, &spellings)
        } else {
            policy.with_rule(&entry.rule.id, &spellings)
        };
    }
    let external = Arc::clone(resolution);
    policy = policy.with_external(move |id| external.is_external(id));
    let reporting = |slug: &str| {
        resolution
            .entry(slug)
            .filter(|entry| entry.active)
            .map(|entry| Severity::from(entry.severity))
    };
    if let Some(level) = reporting(UNUSED) {
        policy = policy.report_unused(level);
    }
    if let Some(level) = reporting(INVALID) {
        policy = policy.report_invalid(level);
    }
    policy
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
}
