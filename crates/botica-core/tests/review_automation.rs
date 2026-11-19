use botica_core::review::automation::authorize;
use serde_json::json;

#[test]
fn authorization_requires_explicit_enablement_and_matching_measured_evidence() {
    let inputs = json!({"base":"b".repeat(40),"head":"a".repeat(40)});
    let report = json!({"inputs":inputs,"mode":"shadow","effective_tier":"low","human_required":false,
        "automation_eligible":true,"merge_performed":false,"checks":{"producer_verified":true,"automation_policy_verified":true,"inputs":inputs},
        "recommendation":"automation-candidate","review_summary":{"inputs":inputs,"status":"complete","verdict":"PASS"},
        "probabilities":{"model":"jev-1.14.0","decision":0.01,"unverifiable":0.01,"contained":0.8,"team":0.19,
            "users":0.01,"confidence":0.99,"request_id":null},
        "question_digest":botica_core::review::questions::digest(),
        "thresholds":{"maximum_question":0.1,"minimum_contained_team":0.9,"minimum_confidence":0.7}});
    let measured = json!({"ready_for_activation":true,"activation_performed":false,
        "question_digest":botica_core::review::questions::digest(),
        "thresholds":{"maximum_question":0.1,"minimum_contained_team":0.9,"minimum_confidence":0.7}});
    assert_eq!(
        authorize(false, &inputs, &report, &measured)["allowed"],
        false
    );
    assert_eq!(
        authorize(true, &inputs, &report, &measured)["allowed"],
        true
    );
    for tier in [json!("medium"), json!("high"), json!(null), json!("LOW")] {
        let mut risky = report.clone();
        risky["effective_tier"] = tier.clone();
        assert_eq!(
            authorize(true, &inputs, &risky, &measured)["allowed"],
            false,
            "{tier}"
        );
    }
    let mut stale = measured;
    stale["question_digest"] = json!("d".repeat(64));
    assert_eq!(authorize(true, &inputs, &report, &stale)["allowed"], false);
    let mut critical = report;
    critical["human_required"] = json!(true);
    assert_eq!(
        authorize(true, &inputs, &critical, &stale)["allowed"],
        false
    );
}
