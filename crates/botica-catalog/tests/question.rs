//! The question of a judgment rule, in the form abide reads.

mod common;

use botica_catalog::{BooleanCriteria, Question, SourceFile};
use common::{catalog, issues, judgment, JUDGMENT_FIELDS};

/// A judgment rule whose question is the table `question`, written after `[implementations.question]`.
fn with_question(question: &str) -> Vec<SourceFile> {
    judgment(
        "",
        &format!("{JUDGMENT_FIELDS}\n[implementations.question]\n{question}"),
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
        &format!("{JUDGMENT_FIELDS}question = \"boolean\"\n"),
    )]);

    assert!(
        message
            .contains("invalid type: string \"boolean\", expected internally tagged enum Question"),
        "{message}"
    );
}
