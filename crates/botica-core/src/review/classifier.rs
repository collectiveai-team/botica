//! Evidence-gated shadow recommendations. This module cannot merge or alter repository settings.

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MODEL: &str = "jev-1.14.0";

/// The classifier and spec reviewer need an explicit acceptance source, not only an issue link.
pub fn has_acceptance_context(context: &Value) -> bool {
    let nonempty = |value: &Value| value.as_str().is_some_and(|text| !text.trim().is_empty());
    nonempty(&context["title"])
        && nonempty(&context["description"])
        && (context["issues"]
            .as_array()
            .is_some_and(|issues| issues.iter().any(|issue| nonempty(&issue["body"])))
            || context["acceptance_criteria"]
                .as_array()
                .is_some_and(|criteria| !criteria.is_empty() && criteria.iter().all(nonempty)))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Probabilities {
    pub model: String,
    pub decision: f64,
    pub unverifiable: f64,
    pub contained: f64,
    pub team: f64,
    pub users: f64,
    pub confidence: f64,
    pub request_id: Option<String>,
}

impl Probabilities {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.model == MODEL, "unexpected classifier model");
        ensure!(
            [
                self.decision,
                self.unverifiable,
                self.contained,
                self.team,
                self.users,
                self.confidence
            ]
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
            "invalid classifier probability"
        );
        ensure!(
            (self.contained + self.team + self.users - 1.0).abs() <= 0.01,
            "impact probabilities do not sum to one"
        );
        Ok(())
    }
}

/// Classify complete change evidence or locate one flagged reason in an individual file.
pub trait Classifier {
    fn classify(&self, state: &Value) -> Result<Probabilities>;
    fn locate(&self, state: &Value, reason: &str) -> Result<f64>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Thresholds {
    pub maximum_question: f64,
    pub minimum_contained_team: f64,
    pub minimum_confidence: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        let settings = botica_config::ClassifierConfig::default();
        Self {
            maximum_question: settings.question_max,
            minimum_contained_team: settings.impact_min,
            minimum_confidence: settings.min_confidence,
        }
    }
}

impl Thresholds {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            [
                self.maximum_question,
                self.minimum_contained_team,
                self.minimum_confidence
            ]
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
            "invalid thresholds"
        );
        ensure!(
            self.maximum_question <= 0.3
                && self.minimum_contained_team >= 0.7
                && self.minimum_confidence >= 0.7,
            "thresholds would treat uncertainty as eligibility"
        );
        Ok(())
    }
}

fn critical(plan: &Value) -> bool {
    ["branch", "integration"].iter().any(|assessment| {
        plan["risk"][assessment]["comparisons"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|comparison| {
                comparison["areas"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|area| area == "critical")
            })
    })
}

fn gates(plan: &Value, results: &Value, checks: &Value) -> Vec<Value> {
    let mut reasons = Vec::new();
    if plan["status"] != "complete" || plan["risk"]["status"] != "complete" {
        reasons.push(json!({"code":"incomplete-risk","source":"gate","evidence":plan["risk"]}));
    }
    if results["inputs"] != plan["inputs"]
        || results["producer_verified"] != true
        || results["status"] != "complete"
        || results["verdict"] != "PASS"
    {
        reasons.push(json!({"code":"review-evidence","source":"gate","evidence":results}));
    }
    if checks["inputs"] != plan["inputs"]
        || checks["producer_verified"] != true
        || checks["ci"] != "PASS"
        || checks["lint"] != "PASS"
        || (plan["mutation_required"] == true && checks["mutation"] != "PASS")
    {
        reasons.push(json!({"code":"check-evidence","source":"gate","evidence":checks}));
    }
    let required = plan["required_checks"].as_array();
    if required.is_none_or(|names| {
        names.is_empty()
            || names.iter().any(|name| {
                name.as_str()
                    .is_none_or(|name| checks["jobs"][name] != "PASS")
            })
    }) {
        reasons.push(json!({"code":"required-checks","source":"gate","evidence":{"required":plan["required_checks"],"observed":checks["jobs"]}}));
    }
    if results["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|finding| matches!(finding["severity"].as_str(), Some("P0" | "P1" | "P2")))
    {
        reasons.push(
            json!({"code":"unresolved-finding","source":"gate","evidence":results["findings"]}),
        );
    }
    reasons
}

fn model_assessment(
    state: &Value,
    classifier: &impl Classifier,
    thresholds: &Thresholds,
) -> Result<(Option<Probabilities>, Vec<Value>)> {
    let serialized = serde_json::to_vec(state)?;
    // UTF-8 byte count bounds tokens conservatively without truncation or omitted files.
    if serialized.len() > 32_000 {
        return Ok((
            None,
            vec![
                json!({"code":"context-limit","source":"gate","bytes":serialized.len(),"limit":32000}),
            ],
        ));
    }
    match classifier.classify(state).and_then(|answer| {
        answer.validate()?;
        Ok(answer)
    }) {
        Ok(answer) => {
            let reasons = [
                ("decision",answer.decision,answer.decision >= thresholds.maximum_question),
                ("unverifiable",answer.unverifiable,answer.unverifiable >= thresholds.maximum_question),
                ("impact-users",answer.users,answer.contained + answer.team < thresholds.minimum_contained_team),
                ("low-confidence",answer.confidence,answer.confidence < thresholds.minimum_confidence),
            ].into_iter().filter(|(_,_,failed)| *failed).map(|(code,probability,_)|
                json!({"code":code,"source":"Jev","probability":probability})).collect();
            Ok((Some(answer), reasons))
        }
        Err(_) => Ok((
            None,
            vec![
                json!({"code":"classifier-unavailable","source":"service","evidence":"Request failed or response was invalid. No model decision was accepted."}),
            ],
        )),
    }
}

/// Evaluate only the model probabilities, for repeated evaluation of recorded responses.
pub fn probabilities_eligible(answer: &Probabilities, thresholds: &Thresholds) -> Result<bool> {
    answer.validate()?;
    thresholds.validate()?;
    Ok(answer.decision < thresholds.maximum_question
        && answer.unverifiable < thresholds.maximum_question
        && answer.contained + answer.team >= thresholds.minimum_contained_team
        && answer.confidence >= thresholds.minimum_confidence)
}

/// Evaluate a shadow candidate. Critical human routing is separate from technical blocking.
pub fn recommend(
    plan: &Value,
    results: &Value,
    checks: &Value,
    state: &Value,
    classifier: &impl Classifier,
    thresholds: &Thresholds,
) -> Result<Value> {
    thresholds.validate()?;
    let human = critical(plan);
    let mut reasons = gates(plan, results, checks);
    if !has_acceptance_context(&state["context"]) {
        reasons.push(json!({"code":"missing-acceptance-context","source":"gate","evidence":"Supply title, description and the linked issue body or explicit acceptance criteria."}));
    }
    if state["context_verified"] != true {
        reasons.push(json!({"code":"context-changed","source":"gate",
            "evidence":"Acceptance context is not verified against current GitHub data."}));
    }
    if state["change_context_complete"] == false {
        reasons.push(json!({"code":"context-unavailable","source":"gate","evidence":"The full change context could not be collected within its bound."}));
    }
    if human {
        reasons.push(json!({"code":"critical-area","source":"gate","evidence":plan["risk"]}));
    }
    let mut probabilities = None;
    if reasons.is_empty() {
        let (answer, found) = model_assessment(state, classifier, thresholds)?;
        probabilities = answer;
        reasons.extend(found);
    }
    if plan["risk"]["effective_tier"]
        .as_str()
        .is_some_and(|tier| tier != "low")
    {
        reasons.push(json!({"code":"risk-not-low","source":"gate","evidence":plan["risk"]["effective_tier"]}));
    }
    let eligible = reasons.is_empty();
    Ok(
        json!({"schema_version":1,"mode":"shadow","inputs":plan["inputs"],"effective_tier":plan["risk"]["effective_tier"],
        "human_required":human,"automation_eligible":eligible,"merge_performed":false,
        "recommendation":if human {"human-review"} else if eligible {"automation-candidate"} else {"blocked"},
        "thresholds":thresholds,"question_digest":super::questions::digest(),"context_verified":state["context_verified"],
        "thresholds_calibrated":false,"probabilities":probabilities,"reasons":reasons}),
    )
}
