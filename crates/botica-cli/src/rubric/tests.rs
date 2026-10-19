use std::sync::Arc;

use serde_json::{json, Value};

use super::*;
use crate::agents_md::tests::{resolution_with, Judgment};

const AGENTS_MD: &str = "# Notes\n\n<!-- botica:rules -->\n### testing\n\n\
    - BTTEST901 `test-one`: Tests assert behaviour through the public seam. → u\n\
    - BTTEST902 `test-turn`: Tests assert behaviour through the public seam. → u\n\
    <!-- /botica:rules -->\n";

const DIRECTIVE: &str =
    "Tests assert behaviour through the public seam. They never read a private \
    helper.\n\nA second paragraph.";

const FILES: [&str; 5] = [
    "src/a.py",
    "src/b.py",
    "src/c.ts",
    "tests/test_a.py",
    "README.md",
];

fn files() -> Vec<String> {
    FILES.iter().map(|file| (*file).to_owned()).collect()
}

fn no_comments(_: &str) -> Option<String> {
    Some(String::new())
}

fn build(rules: &[Judgment], config: &str, read: &dyn Fn(&str) -> Option<String>) -> Value {
    let resolution = Arc::new(resolution_with(rules, config));
    let files = files();
    let inputs = Inputs {
        resolution: &resolution,
        files: &files,
        read,
        agents_md_path: "AGENTS.md",
        agents_md: AGENTS_MD,
    };
    let rubric = rubric(&inputs, "2026-10-15T00:00:00Z".to_owned()).expect("the rubric");
    serde_json::from_str(&render(&rubric)).expect("JSON")
}

const ONE: Judgment = Judgment::new("testing", "test-one", "BTTEST901");

#[test]
fn a_rule_has_its_slug_directive_line_scope_when_and_question() {
    let rubric = build(&[ONE], "", &no_comments);

    assert_eq!(
        rubric,
        json!({
            "version": 1,
            "compiledAt": "2026-10-15T00:00:00Z",
            "compiledBy": format!("botica {}", env!("CARGO_PKG_VERSION")),
            "manual": true,
            "sources": [{ "path": "AGENTS.md" }],
            "rules": [{
                "id": "test-one",
                "text": DIRECTIVE,
                "source": { "path": "AGENTS.md", "line": 6 },
                "scope": ["**/*.py", "**/*.pyi"],
                "when": "edit",
                "check": {
                    "type": "model",
                    "question": { "type": "boolean", "instructions": "q" },
                },
            }],
        })
    );
}

#[test]
fn only_the_active_judgment_rules_judged_on_an_edit_or_a_turn_are_in_the_rubric() {
    let rubric = build(
        &[
            ONE,
            Judgment {
                when: "turn",
                ..Judgment::new("testing", "test-turn", "BTTEST902")
            },
            Judgment {
                when: "pull-request",
                ..Judgment::new("testing", "test-pr", "BTTEST903")
            },
            Judgment {
                enabled: false,
                ..Judgment::new("testing", "test-off", "BTTEST904")
            },
        ],
        "",
        &no_comments,
    );

    let rules: Vec<(&str, &str)> = rubric["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .map(|rule| {
            (
                rule["id"].as_str().expect("an id"),
                rule["when"].as_str().expect("a when"),
            )
        })
        .collect();
    assert_eq!(rules, [("test-one", "edit"), ("test-turn", "turn")]);
    assert_eq!(rubric["rules"][1]["source"]["line"], 7);
}

#[test]
fn the_scope_leaves_out_the_files_that_an_ignore_file_comment_silences() {
    let read = |path: &str| {
        Some(if path == "src/b.py" {
            "# botica: ignore-file[test-one] -- generated code\n".to_owned()
        } else {
            String::new()
        })
    };

    let rubric = build(&[ONE], "", &read);

    assert_eq!(
        rubric["rules"][0]["scope"],
        json!(["src/a.py", "tests/**/*.py", "tests/**/*.pyi"])
    );
}

#[test]
fn the_scope_follows_the_files_of_the_rule_and_the_ignores_of_the_configuration() {
    let rule = Judgment {
        extra: "files = [\"src/**\", \"tests/**\"]",
        ..ONE
    };

    let rubric = build(
        &[rule],
        "[rules.test-one]\nextend-ignores = [\"tests/**\"]\n",
        &no_comments,
    );

    assert_eq!(
        rubric["rules"][0]["scope"],
        json!(["src/**/*.py", "src/**/*.pyi"])
    );
}

#[test]
fn a_rule_of_every_language_covers_every_file() {
    let rule = Judgment {
        languages: "\"*\"",
        ..ONE
    };

    let rubric = build(&[rule], "", &no_comments);

    assert_eq!(rubric["rules"][0]["scope"], json!(["**"]));
}

#[test]
fn a_rule_of_every_language_covers_a_directory_with_files_of_any_language() {
    let rule = Judgment {
        languages: "\"*\"",
        ..ONE
    };

    let rubric = build(
        &[rule],
        "[rules.test-one]\nextend-ignores = [\"README.md\", \"tests/**\"]\n",
        &no_comments,
    );

    assert_eq!(rubric["rules"][0]["scope"], json!(["src/**"]));
}

#[test]
fn a_rule_that_applies_to_no_file_is_left_out() {
    let rubric = build(
        &[ONE],
        "[rules.test-one]\nextend-ignores = [\"**/*.py\"]\n",
        &no_comments,
    );

    assert_eq!(rubric["rules"], json!([]));
}

#[test]
fn a_rule_without_its_line_in_agents_md_is_an_error() {
    let resolution = Arc::new(resolution_with(&[ONE], ""));
    let files = files();
    let inputs = Inputs {
        resolution: &resolution,
        files: &files,
        read: &no_comments,
        agents_md_path: "docs/AGENTS.md",
        agents_md: "<!-- botica:rules -->\n<!-- /botica:rules -->\n",
    };

    let Err(error) = rubric(&inputs, String::new()) else {
        panic!("the rule has no line");
    };

    assert_eq!(
        error,
        "`docs/AGENTS.md` has no line for `test-one`: run `botica rules export --format \
         agents-md --output docs/AGENTS.md` first"
    );
}

fn rendered() -> (String, Value) {
    let rubric = build(&[ONE], "", &no_comments);
    (rubric.to_string(), rubric)
}

fn same(existing: &Value) -> bool {
    let resolution = Arc::new(resolution_with(&[ONE], ""));
    let files = files();
    let inputs = Inputs {
        resolution: &resolution,
        files: &files,
        read: &no_comments,
        agents_md_path: "AGENTS.md",
        agents_md: AGENTS_MD,
    };
    let rubric = rubric(&inputs, "another time".to_owned()).expect("the rubric");
    unchanged(&existing.to_string(), &rubric)
}

#[test]
fn the_check_ignores_what_abide_adds_and_the_time_of_the_export() {
    let (_, mut existing) = rendered();
    existing["compiledAt"] = json!("2020-01-01T00:00:00Z");
    existing["compiledBy"] = json!("abide");
    existing["sources"][0]["sha"] = json!("0".repeat(64));
    existing["rules"][0]["status"] = json!("active");

    assert!(same(&existing));
}

#[test]
fn the_check_sees_a_rule_or_a_source_that_changed() {
    let (text, _) = rendered();
    for (pointer, value) in [
        ("/rules/0/scope", json!(["src/**"])),
        ("/rules/0/status", json!("weak")),
        ("/rules/0/text", json!("other")),
        ("/sources/0/path", json!("CLAUDE.md")),
    ] {
        let mut existing: Value = serde_json::from_str(&text).expect("JSON");
        match existing.pointer_mut(pointer) {
            Some(slot) => *slot = value,
            None => {
                existing["rules"][0]["status"] = value;
            }
        }

        assert!(!same(&existing), "{pointer}");
    }
    assert!(!same(&json!("not a rubric")));
    let resolution = Arc::new(resolution_with(&[ONE], ""));
    let files = files();
    let inputs = Inputs {
        resolution: &resolution,
        files: &files,
        read: &no_comments,
        agents_md_path: "AGENTS.md",
        agents_md: AGENTS_MD,
    };
    let rubric = rubric(&inputs, String::new()).expect("the rubric");
    assert!(!unchanged("{\"sources\": [", &rubric));
}

#[test]
fn compiled_at_is_a_time_of_rfc_3339_in_utc() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(rfc3339(1_700_000_000), "2023-11-14T22:13:20Z");
    assert_eq!(rfc3339(1_798_761_599), "2026-12-31T23:59:59Z");
}
