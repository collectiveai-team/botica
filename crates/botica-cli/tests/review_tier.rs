//! Risk classification through the public CLI with real Git history.

#[path = "review_tier/boundaries.rs"]
mod boundaries;
mod common;
#[path = "review_tier/evidence.rs"]
mod evidence;
#[path = "review_tier/source_ancestry.rs"]
mod source_ancestry;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("Git metadata is UTF-8")
}

fn repository(name: &str, config: Option<&str>) -> std::path::PathBuf {
    let dir = common::project_in("review-tier", name, config);
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Tier test"]);
    git(&dir, &["config", "user.email", "tier@example.com"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "--allow-empty", "-m", "Initial"]);
    git(&dir, &["switch", "-c", "topic"]);
    dir
}

fn classify(dir: &Path, method: &str) -> (std::process::Output, serde_json::Value) {
    let output = common::command(
        dir,
        &[
            "review",
            "tier",
            "--diff",
            "refs/heads/main",
            "--integration-method",
            method,
            "--format",
            "json",
        ],
    );
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", common::stderr(&output)));
    (output, report)
}

const DOCS: &str = r#"
[[review.areas]]
name = "platform"
paths = ["ci/**"]
owners = ["@platform"]
[[review.areas]]
name = "periphery"
paths = ["docs/**"]
"#;

#[test]
fn pilot_source_uncertainty_has_a_conservative_medium_bound_without_invented_facts() {
    let dir = repository("bounded-source", Some(DOCS));
    common::write(&dir, "docs/tool.rs", "pub fn answer() -> u8 { 42 }\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Add pilot source"]);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["effective_tier"], "medium");
    for name in ["branch", "integration"] {
        let comparison = &report[name]["comparisons"][0];
        assert_eq!(comparison["bounds"]["lower"], "low");
        assert_eq!(comparison["bounds"]["upper"], "medium");
        assert_eq!(
            comparison["bounds"]["possible_signals"],
            serde_json::json!(["new-dependency", "new-endpoint"])
        );
        assert_eq!(comparison["detector_evidence"][0]["status"], "unsupported");
        assert_eq!(comparison["signals"], serde_json::json!([]));
        assert_eq!(comparison["signal_evidence"], serde_json::json!([]));
    }
}

#[test]
fn invalid_trusted_review_settings_produce_incomplete_json_with_pinned_inputs() {
    let dir = repository("invalid-review", Some("[review]\nunknown = true\n"));
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["status"], "incomplete");
    assert!(report["effective_tier"].is_null());
    assert_eq!(report["inputs"]["base"].as_str().unwrap().len(), 40);
    assert_eq!(report["inputs"]["config"]["source"], "botica.toml");
    for name in ["branch", "integration"] {
        assert_eq!(report[name]["status"], "incomplete");
        assert_eq!(report[name]["reasons"][0]["code"], "invalid-config");
    }
}

#[test]
fn documentation_uses_destination_configuration_and_records_path_evidence() {
    let config = format!("historical-unknown-setting = true\n{DOCS}");
    let dir = repository("trusted-docs", Some(&config));
    common::write(&dir, "docs/notes.md", "A note.\n");
    common::write(&dir, "botica.toml", "invalid head config");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Document"]);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(report["inputs"]["config"]["source"], "botica.toml");
    assert_eq!(
        report["inputs"]["config"]["digest"],
        blake3::hash(config.as_bytes()).to_hex().to_string()
    );
    let comparison = &report["branch"]["comparisons"][0];
    assert_eq!(
        comparison["files"],
        serde_json::json!(["botica.toml", "docs/notes.md"])
    );
    assert_eq!(
        comparison["areas"],
        serde_json::json!(["periphery", "platform"])
    );
    assert_eq!(comparison["tier"], "medium");
    assert_eq!(
        comparison["evidence"][1]["paths"],
        serde_json::json!(["docs/notes.md"])
    );
    assert_eq!(
        comparison["evidence"][1]["matches"][0]["location"],
        "review.areas[1]"
    );
}

#[test]
fn unsupported_methods_keep_branch_evidence_and_have_no_effective_tier() {
    let dir = repository("unsupported-methods", Some(DOCS));
    common::write(&dir, "docs/note.md", "Note\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Document"]);
    for method in ["squash", "rebase"] {
        let (output, report) = classify(&dir, method);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(report["branch"]["status"], "complete");
        assert_eq!(
            report["branch"]["comparisons"][0]["files"][0],
            "docs/note.md"
        );
        assert_eq!(
            report["integration"]["reasons"][0]["code"],
            "unsupported-integration-method"
        );
        assert!(report["effective_tier"].is_null());
    }
}

#[test]
fn short_tag_names_and_shadowed_branches_are_rejected_even_with_suffixes() {
    let dir = repository("ambiguous-refs", None);
    git(&dir, &["tag", "tagonly"]);
    git(&dir, &["tag", "main"]);
    for (flag, revision) in [
        ("--diff", "main"),
        ("--diff", "main~0"),
        ("--head", "main"),
        ("--diff", "tagonly"),
        ("--diff", "tagonly~0"),
        ("--head", "tagonly^0"),
    ] {
        let output = common::command(
            &dir,
            &[
                "review",
                "tier",
                "--diff",
                "refs/heads/main",
                "--integration-method",
                "merge",
                flag,
                revision,
                "--format",
                "json",
            ],
        );
        assert_eq!(output.status.code(), Some(2), "{flag} {revision}");
        assert!(output.stdout.is_empty());
        assert!(common::stderr(&output).contains(flag));
    }
    let (output, _) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
}

#[test]
fn nul_delimited_paths_preserve_control_characters_unicode_and_rename_sources() {
    let dir = repository("special-paths", Some(DOCS));
    let old = "docs/old \u{b}\\\"é.md";
    let new = "renamed/\u{7}\t\né.md";
    common::write(
        &dir,
        old,
        "A long note that keeps its identity through a rename.\n",
    );
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Base note"]);
    git(&dir, &["branch", "-f", "main", "HEAD"]);
    common::write(
        &dir,
        new,
        "A long note that keeps its identity through a rename.\n",
    );
    std::fs::remove_file(dir.join(old)).unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "Move note"]);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
    let comparison = &report["branch"]["comparisons"][0];
    assert_eq!(comparison["files"], serde_json::json!([old, new]));
    assert_eq!(comparison["tier"], "medium");
    assert_eq!(
        comparison["evidence"][0]["paths"],
        serde_json::json!([old, new])
    );
    assert_eq!(
        comparison["areas"],
        serde_json::json!(["core", "periphery"])
    );
}

#[test]
fn conflicts_preserve_branch_classification_without_touching_user_state() {
    let dir = repository("conflict", Some(DOCS));
    common::write(&dir, "docs/shared.md", "Ancestor\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Ancestor"]);
    git(&dir, &["branch", "-f", "main", "HEAD"]);
    common::write(&dir, "docs/shared.md", "Topic\n");
    git(&dir, &["commit", "-am", "Topic"]);
    git(&dir, &["switch", "main"]);
    common::write(&dir, "docs/shared.md", "Destination\n");
    git(&dir, &["commit", "-am", "Destination"]);
    git(&dir, &["switch", "topic"]);
    common::write(&dir, "docs/staged.md", "Staged\n");
    git(&dir, &["add", "docs/staged.md"]);
    common::write(&dir, "docs/shared.md", "Uncommitted\n");
    let before = git(
        &dir,
        &["status", "--porcelain=v1", "-z", "--untracked-files=no"],
    );
    let refs = git(&dir, &["show-ref"]);
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["branch"]["status"], "complete");
    assert_eq!(
        report["integration"]["reasons"][0]["code"],
        "merge-conflict"
    );
    assert!(report["effective_tier"].is_null());
    assert_eq!(
        git(
            &dir,
            &["status", "--porcelain=v1", "-z", "--untracked-files=no"]
        ),
        before
    );
    assert_eq!(git(&dir, &["show-ref"]), refs);
    assert_eq!(
        std::fs::read_to_string(dir.join("docs/shared.md")).unwrap(),
        "Uncommitted\n"
    );
}

fn criss_cross_repository(
    name: &str,
    auth_path: &str,
    old: &str,
    hardened: &str,
) -> (std::path::PathBuf, String, String) {
    let config = format!("{DOCS}\n[[review.areas]]\nname = \"critical\"\npaths = [\"src/auth/**\"]\nowners = [\"@security\"]\n");
    let dir = repository(name, Some(&config));
    common::write(&dir, auth_path, old);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Old authentication"]);
    let root = git(&dir, &["rev-parse", "HEAD"]).trim().to_owned();
    common::write(&dir, "docs/start.md", "Start\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "P1 docs"]);
    let p1 = git(&dir, &["rev-parse", "HEAD"]).trim().to_owned();
    let old_tree = git(&dir, &["rev-parse", "HEAD^{tree}"]).trim().to_owned();
    git(&dir, &["switch", "--detach", &root]);
    common::write(&dir, auth_path, hardened);
    git(&dir, &["commit", "-am", "P2 authentication"]);
    let p2 = git(&dir, &["rev-parse", "HEAD"]).trim().to_owned();
    common::write(&dir, "docs/start.md", "Start\n");
    git(&dir, &["add", "."]);
    let hardened_tree = git(&dir, &["write-tree"]).trim().to_owned();
    let destination = git(
        &dir,
        &[
            "commit-tree",
            &hardened_tree,
            "-p",
            &p1,
            "-p",
            &p2,
            "-m",
            "Destination merge",
        ],
    )
    .trim()
    .to_owned();
    let evil = git(
        &dir,
        &[
            "commit-tree",
            &old_tree,
            "-p",
            &p1,
            "-p",
            &p2,
            "-m",
            "Evil merge",
        ],
    )
    .trim()
    .to_owned();
    git(&dir, &["branch", "-f", "main", &destination]);
    git(&dir, &["commit", "-m", "Prepare fixture tree"]);
    git(&dir, &["switch", "--detach", &evil]);
    common::write(&dir, "docs/start.md", "Start updated\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Topic documentation"]);
    (dir, p1, p2)
}

#[test]
fn criss_cross_evil_merge_retains_every_ancestor_and_integration_revert() {
    let (dir, p1, p2) = criss_cross_repository(
        "criss-cross",
        "src/auth/check.md",
        "old authentication\n",
        "hardened authentication\n",
    );
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(report["effective_tier"], "high");
    let comparisons = report["branch"]["comparisons"].as_array().unwrap();
    assert_eq!(comparisons.len(), 2);
    assert!(comparisons
        .iter()
        .any(|c| c["from"] == p1 && c["tier"] == "low"));
    assert!(comparisons
        .iter()
        .any(|c| c["from"] == p1 && c["signals"] == serde_json::json!(["docs-only"])));
    assert!(comparisons
        .iter()
        .any(|c| c["from"] == p2 && c["tier"] == "high"));
    assert_eq!(report["integration"]["tier"], "high");
    assert!(report["integration"]["comparisons"][0]["files"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("src/auth/check.md")));
    let blob = git(&dir, &["rev-parse", "main:src/auth/check.md"])
        .trim()
        .to_owned();
    std::fs::remove_file(dir.join(".git/objects").join(&blob[..2]).join(&blob[2..])).unwrap();
    let (output, partial) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    assert!(partial["effective_tier"].is_null());
    assert_eq!(
        partial["branch"]["comparisons"].as_array().unwrap().len(),
        1
    );
    assert_eq!(partial["branch"]["comparisons"][0]["from"], p1);
    assert_eq!(partial["branch"]["reasons"][0]["code"], "unreadable-diff");
}
