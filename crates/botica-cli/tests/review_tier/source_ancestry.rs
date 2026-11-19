use super::*;

#[test]
fn criss_cross_python_revert_has_a_high_policy_bound_with_retained_critical_evidence() {
    let auth_path = "src/auth/check.py";
    let old = "def authenticated(token):\n    return True\n";
    let hardened = "def authenticated(token):\n    return token == 'trusted'\n";
    let (dir, p1, p2) = criss_cross_repository("criss-cross-python", auth_path, old, hardened);
    let ancestors = git(&dir, &["merge-base", "--all", "refs/heads/main", "HEAD"]);
    assert_eq!(ancestors.lines().count(), 2);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["effective_tier"], "high");
    let comparisons = report["branch"]["comparisons"].as_array().unwrap();
    assert_eq!(comparisons.len(), 2);
    assert!(comparisons
        .iter()
        .any(|c| c["from"] == p1 && c["tier"] == "low"));
    let critical = comparisons.iter().find(|c| c["from"] == p2).unwrap();
    assert_eq!(critical["tier"], "high");
    for (name, comparison) in [
        ("branch", critical),
        ("integration", &report["integration"]["comparisons"][0]),
    ] {
        assert_eq!(report[name]["status"], "complete");
        assert_eq!(report[name]["tier"], "high");
        assert_eq!(comparison["bounds"]["lower"], "high");
        assert_eq!(comparison["bounds"]["upper"], "high");
        assert_eq!(comparison["tier"], "high");
        assert!(comparison["areas"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("critical")));
        let file = comparison["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["paths"] == serde_json::json!([auth_path]))
            .unwrap();
        assert!(file["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|area| area["area"] == "critical" && area["tier"] == "high"));
        let detector = comparison["detector_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["paths"] == serde_json::json!([auth_path]))
            .unwrap();
        assert_eq!(detector["status"], "unsupported");
        assert!(comparison["signal_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .all(|signal| signal["name"] != "new-dependency" && signal["name"] != "new-endpoint"));
    }
    assert_eq!(
        report["integration"]["comparisons"][0]["from"],
        report["inputs"]["base"]
    );
    let tree = git(
        &dir,
        &["merge-tree", "--write-tree", "refs/heads/main", "HEAD"],
    );
    assert_eq!(report["integration"]["comparisons"][0]["to"], tree.trim());
    assert_eq!(
        git(&dir, &["show", &format!("{}:{auth_path}", tree.trim())]),
        old
    );
    assert_eq!(
        git(&dir, &["show", &format!("refs/heads/main:{auth_path}")]),
        hardened
    );
}
