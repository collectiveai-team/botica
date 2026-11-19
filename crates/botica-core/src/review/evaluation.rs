//! Reproducible evaluation of labeled changes and recorded, version-bound model observations.

use super::classifier::{probabilities_eligible, Probabilities, Thresholds};
use anyhow::{ensure, Result};
use botica_config::Tier;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCase {
    pub id: String,
    pub source: String,
    pub base: String,
    pub head: String,
    pub label: Option<bool>,
    pub label_source: String,
    pub label_evidence: String,
    pub floor: Option<String>,
    pub critical: Option<bool>,
    pub checks_passed: Option<bool>,
    pub reviews_passed: Option<bool>,
    pub context_complete: Option<bool>,
    #[serde(default)]
    pub inputs_verified: bool,
    pub probabilities: Option<Probabilities>,
    pub question_digest: Option<String>,
}

fn object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn prediction(case: &EvaluationCase, thresholds: &Thresholds) -> Result<Option<bool>> {
    if !case.inputs_verified {
        return Ok(None);
    }
    let (Some(floor), Some(critical), Some(checks), Some(reviews), Some(context)) = (
        case.floor.as_deref(),
        case.critical,
        case.checks_passed,
        case.reviews_passed,
        case.context_complete,
    ) else {
        return Ok(None);
    };
    let Some(floor) = Tier::parse(floor) else {
        anyhow::bail!("invalid evaluation risk floor");
    };
    if !checks || !reviews || !context {
        return Ok(None);
    }
    if floor != Tier::Low || critical {
        return Ok(Some(false));
    }
    case.probabilities
        .as_ref()
        .map(|answer| probabilities_eligible(answer, thresholds))
        .transpose()
}

/// Measure the set without replacing missing labels, stale questions or missing observations.
pub fn evaluate(cases: &[EvaluationCase], thresholds: &Thresholds) -> Result<Value> {
    thresholds.validate()?;
    let mut ids = BTreeSet::new();
    let mut heads = BTreeSet::new();
    let mut evaluated = 0;
    let mut unmeasured = 0;
    let mut unlabeled = 0;
    let mut incompatible = 0;
    let mut candidates = 0;
    let mut false_eligible = 0;
    let mut false_blocked = 0;
    let mut relevant_negative_labels = 0;
    for case in cases {
        ensure!(
            !case.id.trim().is_empty() && ids.insert(&case.id) && heads.insert(&case.head),
            "duplicate evaluation id or change head"
        );
        ensure!(
            object_id(&case.base) && object_id(&case.head) && case.source.starts_with("https://"),
            "evaluation needs pinned Git inputs and a source URL"
        );
        ensure!(
            matches!(
                case.label_source.as_str(),
                "pending" | "confirmed" | "observed-revert"
            ),
            "unknown evaluation label source"
        );
        let Some(label) = case.label else {
            unlabeled += 1;
            continue;
        };
        ensure!(
            case.label_source != "pending"
                && !case.label_evidence.trim().is_empty()
                && (case.label_source != "observed-revert" || !label),
            "label has no supporting evidence"
        );
        if case.probabilities.is_some()
            && case.question_digest.as_deref() != Some(&super::questions::digest())
        {
            incompatible += 1;
            continue;
        }
        let Some(predicted) = prediction(case, thresholds)? else {
            unmeasured += 1;
            continue;
        };
        evaluated += 1;
        candidates += usize::from(predicted);
        false_eligible += usize::from(predicted && !label);
        false_blocked += usize::from(!predicted && label);
        relevant_negative_labels += usize::from(
            !label
                && case.floor.as_deref().and_then(Tier::parse) == Some(Tier::Low)
                && case.critical == Some(false)
                && case.checks_passed == Some(true)
                && case.reviews_passed == Some(true),
        );
    }
    let upper = (candidates > 0).then(|| {
        if false_eligible == 0 {
            1.0 - 0.05_f64.powf(1.0 / candidates as f64)
        } else {
            1.0
        }
    });
    let ready = candidates >= 300
        && relevant_negative_labels >= 30
        && false_eligible == 0
        && upper.is_some_and(|bound| bound <= 0.01)
        && unlabeled + unmeasured + incompatible == 0;
    Ok(
        json!({"schema_version":1,"cases":cases.len(),"evaluated":evaluated,"unlabeled":unlabeled,
        "unmeasured":unmeasured,"incompatible":incompatible,"automation_candidates":candidates,
        "false_eligible":false_eligible,"false_blocked":false_blocked,"relevant_negative_labels":relevant_negative_labels,
        "false_eligibility_upper_95":upper,"question_digest":super::questions::digest(),"thresholds":thresholds,
        "minimum_candidates":300,"minimum_relevant_negatives":30,"maximum_false_eligibility_upper_95":0.01,
        "ready_for_activation":ready,"activation_performed":false,
        "limits":["The confidence bound assumes representative independent cases.",
            "With any false eligibility the reported upper bound is conservatively one.",
            "Readiness never authorizes enabling automation or changing repository settings."]}),
    )
}
