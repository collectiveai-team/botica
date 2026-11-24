//! `review-coverage-current`: the coverage block of a review role against the hooks and jobs it
//! declares, read through an in-memory reader.

use std::collections::BTreeMap;

use botica_core::review_coverage::{check, Role};
use botica_core::Severity;

const PATH: &str = ".github/review/tests/REVIEW.md";
const PREK: &str = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"botica\"\n";
const CI: &str = "jobs:\n  test:\n    runs-on: ubuntu-latest\n";
const FRONT: &str = "---\nrole: tests\ncovered:\n  hooks:\n    - file: prek.toml\n      id: botica\n  \
                     jobs:\n    - file: .github/workflows/ci.yml\n      id: test\n---\n\n## Invariants\n\n";

fn block(prek: &str, ci: &str) -> String {
    format!(
        "<!-- botica:covered -->\n\
         - hook \"botica\" in \"prek.toml\": blake3 {}\n\
         - job \"test\" in \".github/workflows/ci.yml\": blake3 {}\n\
         <!-- /botica:covered -->\n",
        blake3::hash(prek.as_bytes()).to_hex(),
        blake3::hash(ci.as_bytes()).to_hex()
    )
}

fn files(ci: &str) -> BTreeMap<String, Vec<u8>> {
    BTreeMap::from([
        ("prek.toml".to_owned(), PREK.as_bytes().to_vec()),
        (
            ".github/workflows/ci.yml".to_owned(),
            ci.as_bytes().to_vec(),
        ),
    ])
}

fn reported(text: &str, files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let read = |path: &str| Ok(files.get(path).cloned());
    check(&[Role { path: PATH, text }], &read, Severity::Error)
        .into_iter()
        .map(|finding| format!("{}:{}: {}", finding.path, finding.line, finding.message))
        .collect()
}

#[test]
fn a_current_block_has_no_finding() {
    let text = format!("{FRONT}{}", block(PREK, CI));
    assert_eq!(reported(&text, &files(CI)), Vec::<String>::new());
}

#[test]
fn an_edited_job_file_makes_the_block_stale_with_the_lines_to_write() {
    let text = format!("{FRONT}{}", block(PREK, CI));
    let edited = "jobs:\n  test:\n    runs-on: macos-latest\n";
    let found = reported(&text, &files(edited));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with(&format!("{PATH}:14: This coverage block is stale")),
        "{found:?}"
    );
    assert!(
        found[0].contains(block(PREK, edited).trim_end()),
        "{found:?}"
    );
}

#[test]
fn a_source_checked_out_with_crlf_still_matches() {
    let text = format!("{FRONT}{}", block(PREK, CI));
    assert_eq!(
        reported(&text, &files(&CI.replace('\n', "\r\n"))),
        Vec::<String>::new()
    );
}

#[test]
fn without_declared_rules_an_extra_line_in_the_block_is_stale() {
    let text = format!("{FRONT}{}", block(PREK, CI)).replace(
        "<!-- /botica:covered -->",
        "- rule \"file-size-guard\" in \"pyproject.toml\": blake3 0\n<!-- /botica:covered -->",
    );
    assert_eq!(reported(&text, &files(CI)).len(), 1);
}

#[test]
fn with_declared_rules_only_the_hook_and_job_lines_are_compared() {
    let front = FRONT.replace("covered:\n", "covered:\n  rules: [file-size-guard]\n");
    let rule = "- rule \"file-size-guard\" in \"pyproject.toml\": blake3 0\n";
    let current = format!("{front}{}", block(PREK, CI)).replace(
        "<!-- botica:covered -->\n",
        &format!("<!-- botica:covered -->\n{rule}"),
    );
    assert_eq!(reported(&current, &files(CI)), Vec::<String>::new());

    let found = reported(&current, &files("jobs:\n  test: {}\n"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(!found[0].contains("- rule "), "{found:?}");
}

/// The one finding for `text`, which must be at the file.
fn only(text: &str, files: &BTreeMap<String, Vec<u8>>) -> String {
    let found = reported(text, files);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with(&format!("{PATH}:1: ")), "{found:?}");
    found[0].clone()
}

#[test]
fn a_missing_source_is_reported_at_the_file() {
    let mut missing = files(CI);
    missing.remove("prek.toml");
    let found = only(&format!("{FRONT}{}", block(PREK, CI)), &missing);
    assert!(
        found.contains("botica review check cannot verify"),
        "{found}"
    );
    assert!(
        found.contains("missing trusted coverage source prek.toml"),
        "{found}"
    );
}

#[test]
fn a_job_that_its_file_no_longer_has_is_reported() {
    let found = only(
        &format!("{FRONT}{}", block(PREK, CI)),
        &files("jobs:\n  lint: {}\n"),
    );
    assert!(found.contains("missing trusted job test"), "{found}");
}

#[test]
fn a_role_without_covered_is_reported() {
    let legacy = format!("---\nrole: tests\n---\n{}", block(PREK, CI));
    assert!(only(&legacy, &files(CI)).contains("declares no `covered`"));
}

#[test]
fn unbalanced_markers_are_reported() {
    let unbalanced = format!("{FRONT}<!-- botica:covered -->\n");
    assert!(only(&unbalanced, &files(CI)).contains("malformed: unbalanced coverage markers"));
}

#[test]
fn a_role_without_a_block_is_shown_the_block_to_add() {
    let found = only(FRONT, &files(CI));
    assert!(found.contains("has no coverage block"), "{found}");
    assert!(found.contains(block(PREK, CI).trim_end()), "{found}");
}

#[test]
fn a_role_in_the_folder_of_another_is_reported() {
    let wrong = format!(
        "{}{}",
        FRONT.replace("role: tests", "role: spec"),
        block(PREK, CI)
    );
    assert!(only(&wrong, &files(CI)).contains("cannot read this role"));
}
