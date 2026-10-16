use std::collections::BTreeMap;

use botica_catalog::{
    Catalog, Family, Implementation, Kind, Needs, OptionSpec, Question, Rule, RunOn, Runtime,
    Scope, Section, Severity, Status, When,
};
use serde::Serialize;

use crate::rule_page::readme_path;

const SCHEMA_VERSION: u32 = 1;

/// The catalog as one JSON document, for the exports that read it.
#[derive(Serialize)]
struct CatalogView<'a> {
    schema_version: u32,
    families: Vec<&'a Family>,
    rules: Vec<RuleView<'a>>,
}

/// A rule with every field of its `rule.toml`, the releases that added it and gave it its status,
/// the path of its README relative to the catalog, and the sections of the README.
#[derive(Serialize)]
struct RuleView<'a> {
    rule: &'a str,
    code: &'a str,
    family: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog: Option<&'a str>,
    standard: Option<&'a str>,
    summary: &'a str,
    status: Status,
    added: Option<String>,
    since: Option<String>,
    aliases: &'a [String],
    replaces: Option<&'a str>,
    tags: &'a [String],
    files: &'a [String],
    ignores: &'a [String],
    prefilter: Option<&'a str>,
    scope: Option<Scope>,
    needs: Option<Needs>,
    run_on: Option<&'a [RunOn]>,
    default_enabled: bool,
    default_severity: Severity,
    dependencies: &'a [String],
    options: &'a BTreeMap<String, OptionSpec>,
    implementations: Vec<ImplementationView<'a>>,
    readme: String,
    sections: Vec<Section>,
}

/// An implementation with the status it runs with: its own, or the status of its rule.
#[derive(Serialize)]
struct ImplementationView<'a> {
    kind: Kind,
    runtime: Option<Runtime>,
    languages: &'a [String],
    status: Status,
    file: Option<&'a str>,
    when: Option<When>,
    question: Option<&'a Question>,
    covered_by: &'a [String],
    why_not_deterministic: Option<&'a str>,
    tool: Option<&'a str>,
    docs: Option<&'a str>,
    codes: &'a [String],
    config: Option<&'a str>,
}

impl<'a> RuleView<'a> {
    fn new(catalog: &Catalog, rule: &'a Rule) -> Self {
        let lifecycle = catalog.lifecycle(rule);
        Self {
            rule: &rule.id,
            code: &rule.code,
            family: rule.family.as_deref(),
            catalog: rule.catalog.as_deref(),
            standard: rule.standard.as_deref(),
            summary: &rule.summary,
            status: rule.status,
            added: lifecycle.added,
            since: lifecycle.since,
            aliases: &rule.aliases,
            replaces: rule.replaces.as_deref(),
            tags: &rule.tags,
            files: &rule.files,
            ignores: &rule.ignores,
            prefilter: rule.prefilter.as_deref(),
            scope: rule.scope,
            needs: rule.needs,
            run_on: rule.run_on.as_deref(),
            default_enabled: rule.default_enabled,
            default_severity: rule.default_severity,
            dependencies: &rule.dependencies,
            options: &rule.options,
            implementations: rule
                .implementations
                .iter()
                .map(|implementation| ImplementationView::new(rule, implementation))
                .collect(),
            readme: readme_path(rule),
            sections: rule.sections(),
        }
    }
}

impl<'a> ImplementationView<'a> {
    fn new(rule: &Rule, implementation: &'a Implementation) -> Self {
        Self {
            kind: implementation.kind,
            runtime: implementation.runtime,
            languages: &implementation.languages,
            status: rule.status_of(implementation),
            file: implementation.file.as_deref(),
            when: implementation.when,
            question: implementation.question.as_ref(),
            covered_by: &implementation.covered_by,
            why_not_deterministic: implementation.why_not_deterministic.as_deref(),
            tool: implementation.tool.as_deref(),
            docs: implementation.docs.as_deref(),
            codes: &implementation.codes,
            config: implementation.config.as_deref(),
        }
    }
}

pub fn render(catalog: &Catalog) -> String {
    let document = CatalogView {
        schema_version: SCHEMA_VERSION,
        families: catalog.families().iter().collect(),
        rules: catalog
            .rules()
            .iter()
            .map(|rule| RuleView::new(catalog, rule))
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&document).expect("the catalog is JSON");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use botica_catalog::SourceFile;

    fn file(path: &str, text: &str) -> SourceFile {
        SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_rule_promoted_after_its_first_release_says_since_which_release_it_is_stable() {
        let rule = "id = \"no-utils\"\ncode = \"BTSTRUC001\"\nsummary = \"s\"\n\
                    status = \"stable\"\nscope = \"file\"\nneeds = \"path\"\n\
                    run-on = [\"cli\"]\ndefault-enabled = true\ndefault-severity = \"error\"\n\n\
                    [[implementations]]\nkind = \"native\"\nlanguages = [\"*\"]\n";
        let history =
            "[[event]]\nversion = \"0.1.0\"\nrule = \"no-utils\"\nstatus = \"preview\"\n\n\
                       [[event]]\nversion = \"0.2.0\"\nrule = \"no-utils\"\nstatus = \"stable\"\n";
        let catalog = Catalog::from_files(&[
            file(
                "structure/family.toml",
                "code = \"STRUC\"\ndescription = \"d\"\n",
            ),
            file("structure/no-utils/rule.toml", rule),
            file("structure/no-utils/README.md", "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"),
            file("history.toml", history),
        ])
        .expect("the catalog loads");

        let document: serde_json::Value = serde_json::from_str(&render(&catalog)).expect("JSON");

        assert_eq!(document["rules"][0]["added"], "0.1.0");
        assert_eq!(document["rules"][0]["since"], "0.2.0");
    }

    #[test]
    fn the_question_of_a_judgment_rule_is_written_as_abide_reads_it() {
        let rule = "id = \"test-foo\"\ncode = \"BTTEST001\"\nsummary = \"s\"\n\
                    status = \"preview\"\ndefault-enabled = false\ndefault-severity = \"warning\"\n\n\
                    [[implementations]]\nkind = \"judgment\"\nlanguages = [\"python\"]\n\
                    when = \"edit\"\nwhy-not-deterministic = \"w\"\n\n\
                    [implementations.question]\ntype = \"choice\"\ninstructions = \"i\"\n\
                    violating = [\"mock\"]\ncriteria = { real = \"r\", mock = \"m\", fake = \"f\" }\n";
        let catalog = Catalog::from_files(&[
            file(
                "testing/family.toml",
                "code = \"TEST\"\ndescription = \"d\"\n",
            ),
            file("testing/test-foo/rule.toml", rule),
            file("testing/test-foo/README.md", "## Directive\n\nd\n\n## Why\n\nw\n\n## Flag\n\nf\n\n## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"),
        ])
        .expect("the catalog loads");

        let text = render(&catalog);

        let document: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(
            document["rules"][0]["implementations"][0]["question"],
            serde_json::json!({
                "type": "choice",
                "instructions": "i",
                "criteria": { "real": "r", "mock": "m", "fake": "f" },
                "violating": ["mock"],
            })
        );
        assert!(
            text.contains("\"real\": \"r\",\n") && text.find("\"real\"") < text.find("\"fake\""),
            "the options keep their order: {text}"
        );
    }
}
