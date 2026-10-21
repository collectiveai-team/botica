//! A judgment rule: a rule of the catalog that a judge evaluates and `botica lint` never runs.

mod common;

use botica_catalog::{Probability, Question, When};
use common::{
    catalog, family, issues, judgment, judgment_implementation, rule, rule_with, BOOLEAN_QUESTION,
    JUDGMENT_FIELDS, NATIVE_ALL,
};

#[test]
fn a_judgment_rule_loads_with_its_fields_and_none_of_a_deterministic_rule() {
    let loaded = catalog(vec![
        judgment(
            "files = [\"tests/**\"]",
            &format!(
                "{JUDGMENT_FIELDS}covered-by = [\"no-utils\"]\n{}",
                BOOLEAN_QUESTION
            ),
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
    assert_eq!(
        implementation.question,
        Some(Question::Boolean {
            instructions: "Does the test assert through the public seam?".to_owned(),
            criteria: None,
        })
    );
    assert_eq!(implementation.covered_by, ["no-utils"]);
    assert_eq!(
        implementation.why_not_deterministic.as_deref(),
        Some("Whether a seam is public depends on the design of the module.")
    );
}

#[test]
fn a_judgment_implementation_needs_when_question_and_why_not_deterministic() {
    for (field, implementation) in [
        (
            "when",
            judgment_implementation().replace("when = \"edit\"\n", ""),
        ),
        ("question", JUDGMENT_FIELDS.to_owned()),
        (
            "why-not-deterministic",
            judgment_implementation()
                .lines()
                .filter(|line| !line.starts_with("why-not-deterministic"))
                .map(|line| format!("{line}\n"))
                .collect(),
        ),
    ] {
        let message = issues(vec![judgment("", &implementation)]);

        assert!(
            message.contains(&format!("a judgment implementation needs `{field}`")),
            "{field}: {message}"
        );
    }
}

#[test]
fn an_empty_why_not_deterministic_is_rejected() {
    let empty = judgment_implementation().replace(
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
        let message = issues(vec![judgment(line, &judgment_implementation())]);

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
        "question = { type = \"boolean\", instructions = \"i\" }",
        "covered-by = [\"other\"]",
        "why-not-deterministic = \"x\"",
        "act-at = 0.9",
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
fn act_at_is_a_probability_above_0_and_at_most_1() {
    let with = |value: &str| {
        judgment(
            "",
            &format!("{JUDGMENT_FIELDS}act-at = {value}\n{BOOLEAN_QUESTION}"),
        )
    };

    let loaded = catalog(vec![with("0.85")]).unwrap_or_else(|error| panic!("{error}"));
    let rule = loaded.resolve("test-through-interface").expect("the rule");
    assert_eq!(rule.implementations[0].act_at, Some(Probability(0.85)));
    for value in ["0", "1.5"] {
        let message = issues(vec![with(value)]);
        assert!(
            message.contains("`act-at` is a probability above 0 and at most 1"),
            "{value}: {message}"
        );
    }
}

#[test]
fn a_judgment_rule_has_a_single_implementation() {
    let message = issues(vec![judgment(
        "",
        &format!("{}\n{NATIVE_ALL}", judgment_implementation()),
    )]);

    assert!(
        message.contains("a judgment rule has one implementation"),
        "{message}"
    );

    let second = issues(vec![judgment(
        "",
        &format!("{NATIVE_ALL}\n{}", judgment_implementation()),
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
        &format!(
            "{JUDGMENT_FIELDS}covered-by = [\"nope\"]\n{}",
            BOOLEAN_QUESTION
        ),
    )]);
    assert!(
        unknown.contains("covered-by `nope` is not a rule of the catalog"),
        "{unknown}"
    );

    let mut other = judgment("", &judgment_implementation());
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
            &format!(
                "{JUDGMENT_FIELDS}covered-by = [\"test-in-memory-adapters\"]\n{}",
                BOOLEAN_QUESTION
            ),
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
    let mut parts = judgment("", &judgment_implementation());
    parts[2].text = common::README.to_owned();

    let message = issues(vec![parts]);

    assert!(
        message.contains("testing/test-through-interface/README.md: missing section `Flag`"),
        "{message}"
    );
}

fn with_directive(directive: &str) -> Vec<botica_catalog::SourceFile> {
    let mut parts = judgment("", &judgment_implementation());
    parts[2].text = common::JUDGMENT_README.replace(
        "## Directive\n\nd\n",
        &format!("## Directive\n\n{directive}\n"),
    );
    parts
}

#[test]
fn the_directive_of_a_judgment_rule_is_its_text_in_the_rubric_and_has_at_most_600_characters() {
    let words = "word ".repeat(120);
    let fits = words.trim_end();
    assert_eq!(fits.chars().count(), 599);
    let loaded = catalog(vec![with_directive(&format!("{fits}x"))])
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        loaded
            .resolve("test-through-interface")
            .expect("the rule")
            .directive()
            .chars()
            .count(),
        600
    );

    let message = issues(vec![with_directive(&format!("{fits}xy"))]);

    assert!(
        message.contains(
            "testing/test-through-interface/README.md: the Directive of a judgment rule has at \
             most 600 characters, and this one has 601"
        ),
        "{message}"
    );
}

#[test]
fn the_directive_is_read_with_one_space_between_words_and_a_blank_line_between_paragraphs() {
    let loaded = catalog(vec![with_directive(
        "Tests assert   behaviour\nthrough the seam.\n\nA second\n  paragraph.",
    )])
    .unwrap_or_else(|error| panic!("{error}"));

    let rule = loaded.resolve("test-through-interface").expect("the rule");

    assert_eq!(
        rule.directive(),
        "Tests assert behaviour through the seam.\n\nA second paragraph."
    );
}
