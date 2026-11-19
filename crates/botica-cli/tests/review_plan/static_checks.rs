use super::*;

#[test]
fn invalid_frontmatter_and_legacy_coverage_block_static_validation() {
    for (name, role) in [
        (
            "role-mismatch",
            ROLE.replace("role: spec", "role: security"),
        ),
        (
            "unknown-key",
            ROLE.replace("role: spec", "role: spec\nsurprise: true"),
        ),
        ("invalid-tier", ROLE.replace("[low, medium]", "[automatic]")),
        ("invalid-stage", ROLE.replace("[local]", "[production]")),
        (
            "invalid-severity",
            ROLE.replace("role: spec", "role: spec\nblocking: P9"),
        ),
        (
            "zero-findings",
            ROLE.replace("role: spec", "role: spec\nmax-findings: 0"),
        ),
        ("bad-glob", ROLE.replace("docs/**", "!docs/**")),
        (
            "bad-owner",
            ROLE.replace("role: spec", "role: spec\nowners: [not-an-owner]"),
        ),
        (
            "legacy-coverage",
            ROLE.replace("covered:\n  rules: []\n", ""),
        ),
        (
            "traversal",
            ROLE.replace("rules: []", "hooks: [{file: '../prek.toml', id: lint}]"),
        ),
    ] {
        let dir = repository(name, &role);
        common::write(&dir, ".agents/skills/review-spec/SKILL.md", "Available\n");
        let (output, report) = consume(&dir, "check", "local");
        assert_eq!(output.status.code(), Some(2), "{name}: {report}");
        assert_eq!(report["status"], "incomplete");
        assert!(!report["reasons"].as_array().unwrap().is_empty());
    }
}

#[test]
fn missing_declared_jobs_and_unreadable_trusted_roles_are_not_current() {
    let role = ROLE.replace(
        "rules: []",
        "jobs: [{file: '.github/workflows/ci.yml', id: test}]",
    );
    let dir = repository("job-coverage", &role);
    trust(
        &dir,
        ".github/workflows/ci.yml",
        "jobs:\n  other:\n    runs-on: ubuntu-latest\n",
    );
    let (output, report) = consume(&dir, "plan", "local");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["roles"][0]["covered"]["status"], "unverified");
    trust(
        &dir,
        ".github/workflows/ci.yml",
        "jobs:\n  test:\n    runs-on: ubuntu-latest\n",
    );
    let (_, report) = consume(&dir, "plan", "local");
    assert_eq!(report["roles"][0]["covered"]["status"], "stale");
    let current = role.replace(
        "<!-- botica:covered -->\n<!-- /botica:covered -->",
        report["roles"][0]["covered"]["expected_block"]
            .as_str()
            .unwrap(),
    );
    trust(&dir, ".github/review/spec/REVIEW.md", &current);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
}

#[cfg(unix)]
#[test]
fn linked_skills_and_trusted_role_files_are_rejected() {
    let dir = repository("linked-skill", ROLE);
    common::write(&dir, "docs/instructions.md", "Must not be loaded\n");
    std::fs::create_dir_all(dir.join(".agents/skills/review-spec")).unwrap();
    std::os::unix::fs::symlink(
        "../../../docs/instructions.md",
        dir.join(".agents/skills/review-spec/SKILL.md"),
    )
    .unwrap();
    let (output, report) = consume(&dir, "check", "local");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["roles"][0]["skill"], serde_json::Value::Null);
    git(&dir, &["switch", "main"]);
    std::fs::remove_file(dir.join(".github/review/spec/REVIEW.md")).unwrap();
    std::os::unix::fs::symlink(
        "../../../docs/instructions.md",
        dir.join(".github/review/spec/REVIEW.md"),
    )
    .unwrap();
    git(&dir, &["add", ".github/review/spec/REVIEW.md"]);
    git(&dir, &["commit", "-m", "Linked trusted role"]);
    git(&dir, &["switch", "topic"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert_eq!(output.status.code(), Some(2));
    assert!(report["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason["code"] == "invalid-role"));
}
