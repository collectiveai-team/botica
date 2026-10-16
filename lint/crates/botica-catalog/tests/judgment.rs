//! A judgment rule: a rule of the catalog that a judge evaluates and `botica lint` never runs.

mod common;

use botica_catalog::{BooleanCriteria, Question, SourceFile, When};
use common::{catalog, family, file, issues, rule, rule_with, NATIVE_ALL};

const JUDGMENT_README: &str = "## Directive\n\nd\n\n## Why\n\nw\n\n## Flag\n\nf\n\n\
## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
## Suppression\n\ns\n";

const HEAD: &str = "id = \"test-through-interface\"\ncode = \"BTTEST001\"\n\
summary = \"Tests assert behaviour through the public seam\"\nstatus = \"preview\"\n\
default-enabled = false\ndefault-severity = \"warning\"\n";

/// The fields of the root of a judgment implementation, before the table of its question.
macro_rules! fields {
    () => {
        "[[implementations]]\nkind = \"judgment\"\nlanguages = [\"python\"]\nwhen = \"edit\"\n\
         why-not-deterministic = \"Whether a seam is public depends on the design of the module.\"\n"
    };
}

macro_rules! question {
    () => {
        "\n[implementations.question]\ntype = \"boolean\"\n\
         instructions = \"Does the test assert through the public seam?\"\n"
    };
}

const FIELDS: &str = fields!();
const IMPLEMENTATION: &str = concat!(fields!(), question!());

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

/// A judgment rule whose question is the table `question`, written after `[implementations.question]`.
fn with_question(question: &str) -> Vec<SourceFile> {
    judgment(
        "",
        &format!("{FIELDS}\n[implementations.question]\n{question}"),
    )
}

fn question_of(question: &str) -> Question {
    let loaded = catalog(vec![with_question(question)]).unwrap_or_else(|error| panic!("{error}"));
    let rule = loaded.resolve("test-through-interface").expect("the rule");
    rule.implementations[0]
        .question
        .clone()
        .expect("a question")
}

#[test]
fn a_judgment_rule_loads_with_its_fields_and_none_of_a_deterministic_rule() {
    let loaded = catalog(vec![
        judgment(
            "files = [\"tests/**\"]",
            &format!("{FIELDS}covered-by = [\"no-utils\"]\n{}", question!()),
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
fn a_boolean_question_may_describe_what_true_and_false_mean() {
    let question = question_of(
        "type = \"boolean\"\ninstructions = \"i\"\n\
         criteria = { true = \"it asserts through the seam\", false = \"it reads internals\" }\n",
    );

    assert_eq!(
        question,
        Question::Boolean {
            instructions: "i".to_owned(),
            criteria: Some(BooleanCriteria {
                yes: Some("it asserts through the seam".to_owned()),
                no: Some("it reads internals".to_owned()),
            }),
        }
    );
}

#[test]
fn a_choice_question_keeps_its_options_in_the_order_they_are_written() {
    let question = question_of(
        "type = \"choice\"\ninstructions = \"i\"\nviolating = [\"mock\"]\n\
         criteria = { real = \"a real adapter\", fake = \"an in-memory fake\", mock = \"a mock\" }\n",
    );

    let Question::Choice {
        instructions,
        criteria,
        violating,
    } = question
    else {
        panic!("a choice: {question:?}");
    };
    assert_eq!(instructions, "i");
    let options: Vec<(&str, &str)> = criteria
        .0
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    assert_eq!(
        options,
        [
            ("real", "a real adapter"),
            ("fake", "an in-memory fake"),
            ("mock", "a mock")
        ]
    );
    assert_eq!(violating, ["mock"]);
}

#[test]
fn a_score_question_lists_its_levels_and_where_the_violation_starts() {
    let question = question_of(
        "type = \"score\"\ninstructions = \"i\"\ncriteria = [\"none\", \"some\", \"all\"]\n\
         violatingFrom = 2\n",
    );

    assert_eq!(
        question,
        Question::Score {
            instructions: "i".to_owned(),
            criteria: vec!["none".to_owned(), "some".to_owned(), "all".to_owned()],
            violating_from: 2,
        }
    );
}

#[test]
fn a_question_follows_the_schema_of_abide() {
    let long_instructions = "x".repeat(2001);
    let long_criterion = "x".repeat(1001);
    let cases = [
        (
            "type = \"boolean\"\n".to_owned(),
            "missing field `instructions`",
        ),
        (
            "type = \"boolean\"\ninstructions = \"\"\n".to_owned(),
            "implementation 0: question: `instructions` has 1 to 2000 characters",
        ),
        (
            format!("type = \"boolean\"\ninstructions = \"{long_instructions}\"\n"),
            "implementation 0: question: `instructions` has 1 to 2000 characters",
        ),
        (
            format!("type = \"boolean\"\ninstructions = \"i\"\ncriteria = {{ false = \"{long_criterion}\" }}\n"),
            "implementation 0: question: a criterion has at most 1000 characters",
        ),
        (
            "type = \"boolean\"\ninstructions = \"i\"\nextra = 1\n".to_owned(),
            "unknown field `extra`",
        ),
        (
            "type = \"maybe\"\ninstructions = \"i\"\n".to_owned(),
            "unknown variant `maybe`",
        ),
        (
            "type = \"choice\"\ninstructions = \"i\"\nviolating = [\"a\"]\ncriteria = { a = \"x\" }\n"
                .to_owned(),
            "implementation 0: question: a choice question needs at least two options",
        ),
        (
            format!("type = \"choice\"\ninstructions = \"i\"\nviolating = [\"a\"]\ncriteria = {{ a = \"x\", b = \"{long_criterion}\" }}\n"),
            "implementation 0: question: a criterion has at most 1000 characters",
        ),
        (
            "type = \"choice\"\ninstructions = \"i\"\nviolating = [\"a\"]\ncriteria = { a = \"x\", \"\" = \"y\" }\n"
                .to_owned(),
            "implementation 0: question: an option of a choice needs a name",
        ),
        (
            "type = \"choice\"\ninstructions = \"i\"\nviolating = []\ncriteria = { a = \"x\", b = \"y\" }\n"
                .to_owned(),
            "implementation 0: question: a choice question needs at least one violating option",
        ),
        (
            "type = \"choice\"\ninstructions = \"i\"\nviolating = [\"c\"]\ncriteria = { a = \"x\", b = \"y\" }\n"
                .to_owned(),
            "implementation 0: question: violating option `c` is not one of the criteria",
        ),
        (
            "type = \"choice\"\ninstructions = \"i\"\nviolating = [\"a\", \"b\"]\ncriteria = { a = \"x\", b = \"y\" }\n"
                .to_owned(),
            "implementation 0: question: at least one option must be compliant",
        ),
        (
            "type = \"score\"\ninstructions = \"i\"\ncriteria = [\"a\"]\nviolatingFrom = 1\n"
                .to_owned(),
            "implementation 0: question: a score question needs at least two levels",
        ),
        (
            format!("type = \"score\"\ninstructions = \"i\"\ncriteria = [\"a\", \"{long_criterion}\"]\nviolatingFrom = 1\n"),
            "implementation 0: question: a criterion has at most 1000 characters",
        ),
        (
            "type = \"score\"\ninstructions = \"i\"\ncriteria = [\"a\", \"b\"]\nviolatingFrom = 0\n"
                .to_owned(),
            "implementation 0: question: `violatingFrom` is 1 or more and points at one of the levels",
        ),
        (
            "type = \"score\"\ninstructions = \"i\"\ncriteria = [\"a\", \"b\"]\nviolatingFrom = 2\n"
                .to_owned(),
            "implementation 0: question: `violatingFrom` is 1 or more and points at one of the levels",
        ),
    ];
    for (question, expected) in cases {
        let message = issues(vec![with_question(&question)]);

        assert!(message.contains(expected), "{question}: {message}");
    }
}

#[test]
fn a_question_written_as_a_word_is_rejected() {
    let message = issues(vec![judgment(
        "",
        &format!("{FIELDS}question = \"boolean\"\n"),
    )]);

    assert!(
        message
            .contains("invalid type: string \"boolean\", expected internally tagged enum Question"),
        "{message}"
    );
}

#[test]
fn a_judgment_implementation_needs_when_question_and_why_not_deterministic() {
    for (field, implementation) in [
        ("when", IMPLEMENTATION.replace("when = \"edit\"\n", "")),
        ("question", FIELDS.to_owned()),
        (
            "why-not-deterministic",
            IMPLEMENTATION
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
        "question = { type = \"boolean\", instructions = \"i\" }",
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
        &format!("{FIELDS}covered-by = [\"nope\"]\n{}", question!()),
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
            &format!(
                "{FIELDS}covered-by = [\"test-in-memory-adapters\"]\n{}",
                question!()
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
    let mut parts = judgment("", IMPLEMENTATION);
    parts[2].text = common::README.to_owned();

    let message = issues(vec![parts]);

    assert!(
        message.contains("testing/test-through-interface/README.md: missing section `Flag`"),
        "{message}"
    );
}
