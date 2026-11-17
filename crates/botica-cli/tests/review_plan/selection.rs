use super::*;

#[test]
fn an_explicit_empty_tier_list_selects_no_roles() {
    let dir = repository("plan-empty-tiers", &ROLE.replace("[low, medium]", "[]"));
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["selected_roles"], serde_json::json!([]));
}

#[test]
fn stage_filters_complete_plans_but_incomplete_risk_never_selects_roles() {
    let dir = repository("plan-stage", ROLE);
    let (output, report) = consume(&dir, "plan", "ci");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["selected_roles"], serde_json::json!([]));
    assert_eq!(report["roles"][0]["selection"]["stage_matches"], false);
    common::write(&dir, "service.py", "def service():\n    return True\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Unknown source detection"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert!(report["selected_roles"].is_null());
    assert!(report["roles"][0]["selection"]["selected"].is_null());
    assert!(report["risk"]["effective_tier"].is_null());
    assert_eq!(report["reasons"][0]["code"], "incomplete-risk");
    common::write(&dir, ".agents/skills/review-spec/SKILL.md", "Installed\n");
    let (output, report) = consume(&dir, "check", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["command"], "check");
    assert_eq!(report["risk"]["status"], "incomplete");
    assert!(report["selected_roles"].is_null());
}

#[test]
fn lower_tier_ancestor_signals_can_select_a_role_at_the_effective_high_tier() {
    let role = ROLE.replace(
        "tiers: [low, medium]",
        "tiers: [high]\n  signals: [docs-only]",
    );
    let dir = repository("plan-all-ancestors", &role);
    git(&dir, &["switch", "main"]);
    let config = std::fs::read_to_string(dir.join("botica.toml")).unwrap();
    common::write(&dir, "botica.toml", &format!("{config}\n[[review.areas]]\nname = 'critical'\npaths = ['src/auth/**']\nowners = ['@security']\n"));
    common::write(&dir, "src/auth/CODEOWNERS", "old\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Authentication"]);
    let root = git(&dir, &["rev-parse", "HEAD"]);
    common::write(&dir, "docs/start.md", "Start\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "P1 docs"]);
    let p1 = git(&dir, &["rev-parse", "HEAD"]);
    let old_tree = git(&dir, &["rev-parse", "HEAD^{tree}"]);
    git(&dir, &["switch", "--detach", &root]);
    common::write(&dir, "src/auth/CODEOWNERS", "hardened\n");
    git(&dir, &["commit", "-am", "P2 authentication"]);
    let p2 = git(&dir, &["rev-parse", "HEAD"]);
    common::write(&dir, "docs/start.md", "Start\n");
    git(&dir, &["add", "."]);
    let tree = git(&dir, &["write-tree"]);
    let destination = git(
        &dir,
        &[
            "commit-tree",
            &tree,
            "-p",
            &p1,
            "-p",
            &p2,
            "-m",
            "Destination",
        ],
    );
    let evil = git(
        &dir,
        &["commit-tree", &old_tree, "-p", &p1, "-p", &p2, "-m", "Evil"],
    );
    git(&dir, &["branch", "-f", "main", &destination]);
    git(&dir, &["commit", "-m", "Fixture tree"]);
    git(&dir, &["switch", "--detach", &evil]);
    common::write(&dir, "docs/start.md", "Updated\n");
    git(&dir, &["commit", "-am", "Update docs"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["risk"]["effective_tier"], "high");
    assert_eq!(report["selected_roles"], serde_json::json!(["spec"]));
    assert_eq!(
        report["roles"][0]["selection"]["matching_signals"],
        serde_json::json!(["docs-only"])
    );
    let comparisons = report["risk"]["branch"]["comparisons"].as_array().unwrap();
    assert!(comparisons.iter().any(|c| c["from"] == p1
        && c["tier"] == "low"
        && c["signals"] == serde_json::json!(["docs-only"])));
    assert!(comparisons
        .iter()
        .any(|c| c["from"] == p2 && c["tier"] == "high" && c["signals"] == serde_json::json!([])));
}
