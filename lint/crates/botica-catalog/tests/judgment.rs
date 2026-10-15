//! A judgment rule: a rule of the catalog that a judge evaluates and `botica lint` never runs.

mod common;

use botica_catalog::{Question, SourceFile, When};
use common::{catalog, family, file, issues, rule, rule_with, NATIVE_ALL};

const JUDGMENT_README: &str = "## Directive\n\nd\n\n## Why\n\nw\n\n## Flag\n\nf\n\n\
## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
## Suppression\n\ns\n";

const HEAD: &str = "id = \"test-through-interface\"\ncode = \"BTTEST001\"\n\
summary = \"Tests assert behaviour through the public seam\"\nstatus = \"preview\"\n\
default-enabled = false\ndefault-severity = \"warning\"\n";

const IMPLEMENTATION: &str = "[[implementations]]\nkind = \"judgment\"\nlanguages = [\"python\"]\n\
when = \"edit\"\nquestion = \"boolean\"\n\
why-not-deterministic = \"Whether a seam is public depends on the design of the module.\"\n";

fn judgment(head_extra: &str, implementation: &str) -> Vec<SourceFile> {
    vec![
        family("testing", "TEST"),
        file(
            "testing/test-through-interface/rule.toml",
            &format!("{HEAD}{head_extra}\n{implementation}"),
        ),
        file("testing/test-through-interface/README.md", JUDGMENT_README),
    ]
}

#[test]
fn a_judgment_rule_loads_with_its_fields_and_none_of_a_deterministic_rule() {
    let loaded = catalog(vec![
        judgment(
            "files = [\"tests/**\"]",
            &format!("{IMPLEMENTATION}covered-by = [\"no-utils\"]\n"),
        ),
        vec![family("structure", "STRUC")],
        rule("structure", "no-utils", "BTSTRUC001"),
    ])
    .unwrap_or_else(|error| panic!("{error}"));

    let rule = loaded.resolve("test-through-interface").expect("the rule");
    assert_eq!(rule.scope, None);
    assert_eq!(rule.needs, None);
    assert_eq!(rule.run_on, None);
    assert_eq!(rule.files, ["tests/**"]);
    let implementation = &rule.implementations[0];
    assert_eq!(implementation.when, Some(When::Edit));
    assert_eq!(implementation.question, Some(Question::Boolean));
    assert_eq!(implementation.covered_by, ["no-utils"]);
    assert_eq!(
        implementation.why_not_deterministic.as_deref(),
        Some("Whether a seam is public depends on the design of the module.")
    );
}

#[test]
fn a_judgment_implementation_needs_when_question_and_why_not_deterministic() {
    for field in ["when", "question", "why-not-deterministic"] {
        let without: String = IMPLEMENTATION
            .lines()
            .filter(|line| !line.starts_with(field))
            .map(|line| format!("{line}\n"))
            .collect();

        let message = issues(vec![judgment("", &without)]);

        assert!(
            message.contains(&format!("a judgment implementation needs `{field}`")),
            "{field}: {message}"
        );
    }
}

#[test]
fn an_empty_why_not_deterministic_is_rejected() {
    let empty = IMPLEMENTATION.replace(
        "\"Whether a seam is public depends on the design of the module.\"",
        "\"  \"",
    );

    let message = issues(vec![judgment("", &empty)]);

    assert!(
        message.contains("a judgment implementation needs `why-not-deterministic`"),
        "{message}"
    );
}

#[test]
fn a_judgment_rule_has_no_scope_needs_run_on_or_prefilter() {
    for (field, line) in [
        ("scope", "scope = \"file\""),
        ("needs", "needs = \"syntax\""),
        ("run-on", "run-on = [\"cli\"]"),
        ("prefilter", "prefilter = \"x\""),
    ] {
        let message = issues(vec![judgment(line, IMPLEMENTATION)]);

        assert!(
            message.contains(&format!(
                "`{field}` applies to a deterministic rule, and this one is judgment"
            )),
            "{field}: {message}"
        );
    }
}

#[test]
fn a_deterministic_rule_needs_scope_needs_and_run_on() {
    for field in ["scope", "needs", "run-on"] {
        let mut parts = rule("structure", "no-utils", "BTSTRUC001");
        parts[0].text = parts[0]
            .text
            .lines()
            .filter(|line| !line.starts_with(field))
            .map(|line| format!("{line}\n"))
            .collect();

        let message = issues(vec![vec![family("structure", "STRUC")], parts]);

        assert!(
            message.contains(&format!("a deterministic rule needs `{field}`")),
            "{field}: {message}"
        );
    }
}

#[test]
fn the_fields_of_a_judgment_belong_to_a_judgment_implementation_only() {
    for line in [
        "when = \"edit\"",
        "question = \"score\"",
        "covered-by = [\"other\"]",
        "why-not-deterministic = \"x\"",
    ] {
        let field = line.split(' ').next().expect("a field");
        let message = issues(vec![
            vec![family("structure", "STRUC")],
            rule_with(
                "structure",
                "no-utils",
                "BTSTRUC001",
                "",
                &format!("{NATIVE_ALL}{line}\n"),
            ),
        ]);

        assert!(
            message.contains(&format!(
                "implementation 0: `{field}` applies to a judgment implementation only"
            )),
            "{field}: {message}"
        );
    }
}

#[test]
fn a_judgment_rule_has_a_single_implementation() {
    let message = issues(vec![judgment(
        "",
        &format!("{IMPLEMENTATION}\n{NATIVE_ALL}"),
    )]);

    assert!(
        message.contains("a judgment rule has one implementation"),
        "{message}"
    );

    let second = issues(vec![judgment(
        "",
        &format!("{NATIVE_ALL}\n{IMPLEMENTATION}"),
    )]);
    assert!(
        second.contains("a judgment rule has one implementation"),
        "{second}"
    );
}

#[test]
fn covered_by_names_deterministic_rules_of_the_catalog() {
    let unknown = issues(vec![judgment(
        "",
        &format!("{IMPLEMENTATION}covered-by = [\"nope\"]\n"),
    )]);
    assert!(
        unknown.contains("covered-by `nope` is not a rule of the catalog"),
        "{unknown}"
    );

    let mut other = judgment("", IMPLEMENTATION);
    other.remove(0);
    for part in &mut other {
        part.path = part
            .path
            .replace("test-through-interface", "test-in-memory-adapters");
        part.text = part
            .text
            .replace("test-through-interface", "test-in-memory-adapters")
            .replace("BTTEST001", "BTTEST002");
    }
    let judged = issues(vec![
        judgment(
            "",
            &format!("{IMPLEMENTATION}covered-by = [\"test-in-memory-adapters\"]\n"),
        ),
        other,
    ]);
    assert!(
        judged.contains("covered-by `test-in-memory-adapters` is a judgment rule"),
        "{judged}"
    );
}

#[test]
fn the_readme_of_a_judgment_rule_follows_the_template_of_its_kind() {
    let mut parts = judgment("", IMPLEMENTATION);
    parts[2].text = common::README.to_owned();

    let message = issues(vec![parts]);

    assert!(
        message.contains("testing/test-through-interface/README.md: missing section `Flag`"),
        "{message}"
    );
}
