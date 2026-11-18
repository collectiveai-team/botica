use botica_core::review::state::build;
use serde_json::json;

#[test]
fn classifier_diff_excludes_generated_content_but_retains_literal_rename_paths() {
    let comparison = json!({"from":"a","to":"b","evidence":[
        {"generated":true,"kind":"binary","paths":["generated/large.bin"]},
        {"generated":false,"kind":"text","paths":["[literal].rs","new name.rs"]}]});
    let plan = json!({"risk":{"branch":{"comparisons":[comparison.clone()]},
        "integration":{"comparisons":[comparison]}}});
    let selections = botica_core::review::state::diff_selections(&plan).unwrap();
    assert_eq!(selections.len(), 1);
    assert_eq!(selections[0].paths, ["[literal].rs", "new name.rs"]);
    let only_generated = json!({"risk":{"branch":{"comparisons":[
        {"from":"a","to":"b","evidence":[{"generated":true,"kind":"binary","paths":["generated/a"]}]}]}}});
    let selections = botica_core::review::state::diff_selections(&only_generated).unwrap();
    assert!(selections[0].paths.is_empty());
    let unsupported = json!({"risk":{"branch":{"comparisons":[
        {"from":"a","to":"b","evidence":[{"generated":false,"kind":"binary","paths":["input/a"]}]}]}}});
    assert!(botica_core::review::state::diff_selections(&unsupported).is_err());
}

#[test]
fn classifier_state_preserves_decision_content_without_gate_metadata_or_duplicate_findings() {
    let comparison = json!({"evidence":[{"paths":["src/a.rs"],"kind":"text","status":"M","generated":false,
        "matches":[{"area":"core","tier":"medium","location":"review.areas[0]"}]}],
        "signal_evidence":[{"signal":"new-endpoint","paths":["src/a.rs"],"detector":"test","detail":"route"}]});
    let plan = json!({"untrusted_context":{"title":"Title","description":"Description",
        "issues":[{"body":"Required behaviour"}],"acceptance_criteria":["Expected"]},
        "inputs":{"secret_metadata":"provenance only"},"roles":[{"definition":{"path":"policy"}}],
        "selected_roles":["tests"],"risk":{"effective_tier":"medium",
        "branch":{"comparisons":[comparison.clone()]},"integration":{"comparisons":[comparison]}}});
    let reviews = json!({"status":"complete","verdict":"PASS","provenance":{"artifact":"metadata"},
        "findings":[{"id":"tests-1","evidence":"Meaningful finding"}],
        "all_findings":[{"id":"tests-1","evidence":"Meaningful finding"}],
        "roles":[{"role":"tests","verdict":"PASS","not_checked":["external service"],
        "inputs":{"head":"provenance"},"findings":[{"id":"tests-1"}]}]});
    let checks = json!({"ci":"PASS","lint":"PASS","mutation":"PASS","jobs":{"unit":"PASS"},
        "provenance":{"run_id":123}});
    let originals = (plan.clone(), reviews.clone(), checks.clone());
    let state = build(&plan, &reviews, &checks, json!([{"diff":"+changed"}]), true);
    assert!(state.get("plan").is_none());
    assert!(state.get("reviews").is_none());
    assert_eq!(state["context"]["issues"][0]["body"], "Required behaviour");
    assert_eq!(state["summary"]["findings"], reviews["findings"]);
    assert_eq!(state["summary"]["checks"]["mutation"], "PASS");
    assert_eq!(
        state["summary"]["roles"][0]["not_checked"][0],
        "external service"
    );
    assert_eq!(state["files"].as_array().unwrap().len(), 1);
    assert_eq!(state["signals"].as_array().unwrap().len(), 1);
    assert_eq!(state["files"][0]["areas"], json!(["core"]));
    assert_eq!(state["context_verified"], true);
    assert_eq!(state["change_context_complete"], true);
    assert!(!state.to_string().contains("provenance"));
    assert_eq!((plan, reviews, checks), originals);
}
