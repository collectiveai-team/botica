//! GitHub context ingestion through a working fake of the external gh client.
#![cfg(unix)]

mod common;
use serde_json::{json, Value};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn context(dir: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args([
            "review",
            "context",
            "--repository",
            "owner/repo",
            "--pr",
            "42",
            "--base",
            &"b".repeat(40),
            "--head",
            &"a".repeat(40),
        ])
        .current_dir(dir)
        .env(
            "PATH",
            format!(
                "{}:{}",
                dir.join(".tmp/bin").display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("FAKE_GH_ROOT", dir)
        .env("OPENAI_API_KEY", "synthetic-test-key")
        .output()
        .unwrap()
}

fn fixture(name: &str, truncated: bool) -> std::path::PathBuf {
    let dir = common::project_in("github-context", name, None);
    common::write(
        &dir,
        ".tmp/bin/gh",
        r#"#!/bin/sh
test -z "${OPENAI_API_KEY:-}" || exit 9
if test "$2" = graphql; then
  cat "$FAKE_GH_ROOT/.tmp/issues.json"
else
  cat "$FAKE_GH_ROOT/.tmp/pull.json"
fi
"#,
    );
    std::fs::set_permissions(
        dir.join(".tmp/bin/gh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    common::write(
        &dir,
        ".tmp/pull.json",
        &json!({"state":"open","draft":false,
        "base":{"sha":"b".repeat(40)},"head":{"sha":"a".repeat(40)},
        "title":"Return 42","body":"Closes #7"})
        .to_string(),
    );
    common::write(&dir,".tmp/issues.json",&json!({"data":{"repository":{"pullRequest":{"closingIssuesReferences":{
        "nodes":[{"number":7,"title":"Decided answer","body":"The public answer must be 42","url":"https://github.com/owner/repo/issues/7"}],
        "pageInfo":{"hasNextPage":truncated}}}}}}).to_string());
    dir
}

#[test]
fn linked_issue_bodies_are_delivered_without_giving_gh_the_model_credential() {
    let dir = fixture("complete", false);
    let output = context(&dir);
    assert!(output.status.success(), "{}", common::stderr(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["issues"][0]["body"], "The public answer must be 42");
    assert_eq!(report["head"], "a".repeat(40));
}

#[test]
fn truncated_issue_context_is_rejected_instead_of_accepted_as_complete() {
    let dir = fixture("truncated", true);
    let output = context(&dir);
    assert_eq!(output.status.code(), Some(2));
    assert!(common::stderr(&output).contains("linked issue context is incomplete"));
}

#[test]
fn explicit_acceptance_criteria_in_the_pr_description_survive_without_a_linked_issue() {
    let dir = fixture("description-criteria", false);
    common::write(&dir, ".tmp/pull.json", &json!({"state":"open","draft":false,
        "base":{"sha":"b".repeat(40)},"head":{"sha":"a".repeat(40)},
        "title":"Return 42","body":"Implement the answer\n\n## Acceptance criteria\n- The answer is 42\n\n## Notes\nNot a criterion"}).to_string());
    common::write(
        &dir,
        ".tmp/issues.json",
        &json!({"data":{"repository":{"pullRequest":{"closingIssuesReferences":{
        "nodes":[],"pageInfo":{"hasNextPage":false}}}}}})
        .to_string(),
    );
    let output = context(&dir);
    assert!(output.status.success(), "{}", common::stderr(&output));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["acceptance_criteria"], json!(["- The answer is 42"]));
    assert!(botica_core::review::classifier::has_acceptance_context(
        &report
    ));
}
