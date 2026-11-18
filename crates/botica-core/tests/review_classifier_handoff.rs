use botica_core::review::classifier::handoff_markdown;
use serde_json::json;

#[test]
fn the_published_handoff_includes_localized_paths_probabilities_and_unavailable_evidence() {
    let report = json!({"recommendation":"blocked","human_required":false,
        "deterministic_tier":"medium","effective_tier":"high","raised_by_classifier":true,
        "probabilities":{"decision":0.8},"risk_evidence":{"branch":{"signals":["new-endpoint"]}},
        "inputs":{"head":"a".repeat(40)},"decision_requested":"resolve-evidence-blockers",
        "review_summary":{"roles":[{"role":"spec","verdict":"PASS"}]},
        "not_reviewed":[{"role":"security","reason":"not selected by destination policy"}],
        "localization":[{"reason":"decision","path":"src/answer.rs","probability":0.82,"status":"measured"},
            {"reason":"unverifiable","path":"src/other.rs","probability":null,"status":"unavailable"}],
        "reasons":[{"code":"decision","source":"Jev","probability":0.8,
            "verify":"Compare the linked criterion.","resolve":"Record the intended choice."}]});
    let text = handoff_markdown(&report);
    assert!(text.starts_with("<!-- botica:review-handoff -->"));
    assert!(text.contains("\"effective_tier\":\"high\""));
    assert!(text.contains("\"deterministic_tier\":\"medium\""));
    assert!(text.contains("\"decision\":0.8"));
    assert!(text.contains("new-endpoint"));
    let localization = text
        .split("## Localization\n")
        .nth(1)
        .unwrap()
        .split("\n## decision")
        .next()
        .unwrap();
    assert!(localization.contains("src/answer.rs"));
    assert!(localization.contains("0.82"));
    assert!(localization.contains("src/other.rs"));
    assert!(localization.contains("\"status\":\"unavailable\""));
    assert!(text.contains("Compare the linked criterion."));
    assert!(text.contains("Record the intended choice."));
    assert!(text.contains("not selected by destination policy"));
}
