//! Authorization data only. The caller owns any GitHub operation; this module never merges.

use super::classifier::{probabilities_eligible, Probabilities, Thresholds};
use serde_json::{json, Value};

/// Require fresh evidence and a matching measured policy, even after explicit enablement.
pub fn authorize(enabled: bool, inputs: &Value, report: &Value, evaluation: &Value) -> Value {
    let thresholds = serde_json::from_value::<Thresholds>(report["thresholds"].clone()).ok();
    let probabilities =
        serde_json::from_value::<Probabilities>(report["probabilities"].clone()).ok();
    let model_passed =
        thresholds
            .as_ref()
            .zip(probabilities.as_ref())
            .is_some_and(|(thresholds, answer)| {
                probabilities_eligible(answer, thresholds).unwrap_or(false)
            });
    let evidence = report["inputs"] == *inputs
        && report["mode"] == "shadow"
        && report["effective_tier"] == "low"
        && report["human_required"] == false
        && report["automation_eligible"] == true
        && report["recommendation"] == "automation-candidate"
        && report["merge_performed"] == false
        && report["checks"]["producer_verified"] == true
        && report["checks"]["automation_policy_verified"] == true
        && report["checks"]["inputs"] == *inputs
        && report["review_summary"]["inputs"] == *inputs
        && report["review_summary"]["status"] == "complete"
        && report["review_summary"]["verdict"] == "PASS"
        && model_passed;
    let measured = evaluation["ready_for_activation"] == true
        && evaluation["activation_performed"] == false
        && evaluation["question_digest"] == report["question_digest"]
        && report["question_digest"] == super::questions::digest()
        && evaluation["thresholds"] == report["thresholds"];
    json!({"schema_version":1,"allowed":enabled && evidence && measured,"inputs":inputs,
        "enabled":enabled,"evidence_valid":evidence,"evaluation_ready":measured,
        "merge_performed":false,"human_required":report["human_required"],
        "reason":if !enabled {"automation-disabled"} else if !evidence {"invalid-current-evidence"} else if !measured {"evaluation-not-ready"} else {"authorized-current-inputs"}})
}
