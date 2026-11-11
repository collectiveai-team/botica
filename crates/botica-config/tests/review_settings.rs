//! The `review` table of the configuration besides the areas: generated files, the folder and the
//! roles, and the classifier (ADR 0017 and ADR 0018).

mod common;

use botica_config::{ClassifierMode, ReviewRole};
use common::review::{accepted, problems, with_platform, PLATFORM};

#[test]
fn a_generated_glob_cannot_match_a_path_that_a_person_has_to_review() {
    for glob in ["**", ".github/**", "**/*.toml", "ci/**", "db/**", "*.md"] {
        let message = problems(&format!(
            "[review]\ngenerated = [\"{glob}\"]\n\n{}",
            with_platform(
                "[[review.areas]]\nname = \"critical\"\npaths = [\"db/migrations/**\"]\nowners = [\"@o\"]\n"
            )
        ));
        assert!(
            message.contains("review.generated[0]"),
            "`{glob}`: {message}"
        );
    }
    accepted(&format!(
        "[review]\ngenerated = [\"/uv.lock\", \"schema/botica.schema.json\", \"src/gen/api.pb.go\"]\n\n{}",
        with_platform("[[review.areas]]\nname = \"core\"\npaths = [\"src/**\"]\n")
    ));
}

#[test]
fn a_generated_glob_must_be_a_glob() {
    let message = problems("[review]\ngenerated = [\"{a\"]\n");

    assert!(
        message.contains("review.generated[0]: invalid glob"),
        "{message}"
    );
    for blank in ["", " ", "#a", "!a"] {
        let message = problems(&format!("[review]\ngenerated = [\"{blank}\"]\n"));
        assert!(
            message.contains("review.generated[0]") && message.contains("is not a glob of paths"),
            "`{blank}`: {message}"
        );
    }
    let message = problems("[review]\ngenerated = [\" a\"]\n");
    assert!(message.contains("has spaces around it"), "{message}");
}

#[test]
fn the_classifier_takes_the_bounds_of_the_adr() {
    let config = accepted(
        "[review.classifier]\nmode = \"enforce\"\nrequired-checks = [\"test\"]\nquestion-max = 0.3\n\
         impact-min = 0.75\nmin-confidence = 0.8\nmax-changed-lines = 1000\n",
    );

    let classifier = &config.review.classifier;
    assert_eq!(classifier.mode, ClassifierMode::Enforce);
    assert_eq!(classifier.required_checks, ["test"]);
    assert_eq!(classifier.question_max, 0.3);
    assert_eq!(classifier.impact_min, 0.75);
    assert_eq!(classifier.min_confidence, 0.8);
    assert_eq!(classifier.max_changed_lines, 1000);
    for (key, value) in [
        ("question-max", "0.31"),
        ("question-max", "-0.1"),
        ("question-max", "nan"),
        ("impact-min", "0.69"),
        ("impact-min", "1.1"),
        ("min-confidence", "0.69"),
        ("min-confidence", "1.5"),
        ("max-changed-lines", "1001"),
    ] {
        let message = problems(&format!("[review.classifier]\n{key} = {value}\n"));
        assert!(
            message.contains(&format!("review.classifier.{key}")),
            "{key} = {value}: {message}"
        );
    }
}

#[test]
fn a_required_check_has_a_name() {
    let message = problems("[review.classifier]\nrequired-checks = [\"test\", \" \"]\n");

    assert!(
        message.contains("review.classifier.required-checks[1]"),
        "{message}"
    );
}

#[test]
fn the_folder_of_the_roles_is_a_plain_directory() {
    accepted("[review]\ndir = \"policy/review\"\n");
    for bad in [
        "",
        "/policy",
        "policy/",
        "./policy",
        "../policy",
        "a//b",
        "[",
        "*",
        "po*cy",
        "!x",
        "#r",
        "a?b",
        "{a,b}",
    ] {
        let message = problems(&format!("[review]\ndir = \"{bad}\"\n"));
        assert!(message.contains("review.dir"), "`{bad}`: {message}");
    }
}

#[test]
fn the_roles_that_ci_expects_are_the_ones_the_table_names() {
    let config = accepted("[review]\nroles = [\"spec\", \"security\", \"refute\"]\n");

    assert_eq!(
        config.review.expected_roles(),
        [ReviewRole::Spec, ReviewRole::Security, ReviewRole::Refute]
    );
    assert!(accepted("[review]\nroles = []\n")
        .review
        .expected_roles()
        .is_empty());
}

#[test]
fn the_globs_that_match_no_file_are_listed_with_where_they_are_written() {
    let config = accepted(&format!(
        "[review]\ngenerated = [\"/uv.lock\", \"/gone.lock\"]\n\n{PLATFORM}\n\
         [[review.areas]]\nname = \"core\"\npaths = [\"src/**\", \"old/**\"]\n"
    ));
    let files = ["ci/build.yml", "src/main.rs", "uv.lock", "README.md"];

    let unmatched = config.review.unmatched_paths(&files);

    let found: Vec<(&str, &str)> = unmatched
        .iter()
        .map(|path| (path.location.as_str(), path.glob.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            ("review.generated[1]", "/gone.lock"),
            ("review.areas[1].paths[1]", "old/**"),
        ]
    );
    assert_eq!(config.review.unmatched_paths(&[]).len(), 5);
}
