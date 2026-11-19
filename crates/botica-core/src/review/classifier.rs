//! Evidence-gated shadow recommendations. This module cannot merge or alter repository settings.

use super::reasons::ReasonCode;
use anyhow::{ensure, Context, Result};
use botica_config::Tier;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MODEL: &str = "jev-1.14.0";

/// Maximum reported input-token usage accepted from the pinned model.
pub const INPUT_TOKEN_LIMIT: u64 = 32_000;
/// Independent local memory bound for serialized model requests.
pub const REQUEST_BYTE_LIMIT: usize = 256 * 1024;

/// A measured model response exceeds the accepted input-token budget.
#[derive(Debug)]
pub struct ContextLimit {
    pub input_tokens: u64,
}

impl std::fmt::Display for ContextLimit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "model input exceeds the accepted token budget")
    }
}

impl std::error::Error for ContextLimit {}

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
    if plan["status"] != "complete"
        || plan["risk"]["status"] != "complete"
        || plan["risk"]["effective_tier"]
            .as_str()
            .and_then(Tier::parse)
            .is_none()
    {
        reasons.push(
            json!({"code":ReasonCode::IncompleteRisk,"source":"gate","evidence":plan["risk"]}),
        );
    }
    if results["inputs"] != plan["inputs"]
        || results["producer_verified"] != true
        || results["status"] != "complete"
        || results["verdict"] != "PASS"
    {
        reasons.push(json!({"code":ReasonCode::ReviewEvidence,"source":"gate","evidence":results}));
    }
    if checks["inputs"] != plan["inputs"]
        || checks["producer_verified"] != true
        || checks["ci"] != "PASS"
        || checks["lint"] != "PASS"
        || (plan["mutation_required"] == true && checks["mutation"] != "PASS")
    {
        reasons.push(json!({"code":ReasonCode::CheckEvidence,"source":"gate","evidence":checks}));
    }
    let required = plan["required_checks"].as_array();
    if required.is_none_or(|names| {
        names.is_empty()
            || names.iter().any(|name| {
                name.as_str()
                    .is_none_or(|name| checks["jobs"][name] != "PASS")
            })
    }) {
        reasons.push(json!({"code":ReasonCode::RequiredChecks,"source":"gate","evidence":{"required":plan["required_checks"],"observed":checks["jobs"]}}));
    }
    if results["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|finding| matches!(finding["severity"].as_str(), Some("P0" | "P1" | "P2")))
    {
        reasons.push(
            json!({"code":ReasonCode::UnresolvedFinding,"source":"gate","evidence":results["findings"]}),
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
    if serialized.len() > REQUEST_BYTE_LIMIT {
        return Ok((
            None,
            vec![
                json!({"code":ReasonCode::ContextByteLimit,"source":"gate","bytes":serialized.len(),"limit":REQUEST_BYTE_LIMIT}),
            ],
        ));
    }
    match classifier.classify(state).and_then(|answer| {
        answer.validate()?;
        Ok(answer)
    }) {
        Ok(answer) => {
            let reasons = [
                (ReasonCode::Decision,answer.decision,answer.decision >= thresholds.maximum_question),
                (ReasonCode::Unverifiable,answer.unverifiable,answer.unverifiable >= thresholds.maximum_question),
                (ReasonCode::ImpactUsers,answer.users,answer.contained + answer.team < thresholds.minimum_contained_team),
                (ReasonCode::LowConfidence,answer.confidence,answer.confidence < thresholds.minimum_confidence),
            ].into_iter().filter(|(_,_,failed)| *failed).map(|(code,probability,_)|
                json!({"code":code,"source":"Jev","probability":probability})).collect();
            Ok((Some(answer), reasons))
        }
        Err(error) if error.downcast_ref::<ContextLimit>().is_some() => {
            let limit = error.downcast_ref::<ContextLimit>().unwrap();
            Ok((
                None,
                vec![json!({"code":ReasonCode::ContextLimit,"source":"service",
                "input_tokens":limit.input_tokens,"limit_tokens":INPUT_TOKEN_LIMIT})],
            ))
        }
        Err(_) => Ok((
            None,
            vec![
                json!({"code":ReasonCode::ClassifierUnavailable,"source":"service","evidence":"Request failed or response was invalid. No model decision was accepted."}),
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
        reasons.push(json!({"code":ReasonCode::MissingAcceptanceContext,"source":"gate","evidence":"Supply title, description and the linked issue body or explicit acceptance criteria."}));
    }
    if state["context_verified"] != true {
        reasons.push(json!({"code":ReasonCode::ContextChanged,"source":"gate",
            "evidence":"Acceptance context is not verified against current GitHub data."}));
    }
    if state["change_context_complete"] != true {
        reasons.push(json!({"code":ReasonCode::ContextUnavailable,"source":"gate","evidence":"The full change context could not be collected within its bound."}));
    }
    if human {
        reasons
            .push(json!({"code":ReasonCode::CriticalArea,"source":"gate","evidence":plan["risk"]}));
    }
    let mut probabilities = None;
    if reasons.is_empty() {
        let (answer, found) = model_assessment(state, classifier, thresholds)?;
        probabilities = answer;
        reasons.extend(found);
    }
    let floor = &plan["risk"]["effective_tier"];
    let floor_tier = floor.as_str().and_then(Tier::parse);
    let raised = floor_tier.is_some_and(|tier| tier < Tier::High)
        && reasons.iter().any(|reason| {
            reason["source"] == "Jev"
                && reason["code"]
                    .as_str()
                    .and_then(ReasonCode::parse)
                    .is_some_and(ReasonCode::raises_risk)
        });
    let effective = match floor_tier {
        Some(tier) if raised => json!(tier.raised()),
        _ => floor.clone(),
    };
    if effective
        .as_str()
        .is_some_and(|tier| Tier::parse(tier) != Some(Tier::Low))
    {
        reasons.push(json!({"code":ReasonCode::RiskNotLow,"source":"gate","evidence":effective}));
    }
    let eligible = reasons.is_empty();
    Ok(
        json!({"schema_version":1,"mode":"shadow","inputs":plan["inputs"],"deterministic_tier":floor,
        "effective_tier":effective,"raised_by_classifier":raised,
        "risk_evidence":plan["risk"],
        "human_required":human,"automation_eligible":eligible,"merge_performed":false,
        "recommendation":if human {"human-review"} else if eligible {"automation-candidate"} else {"blocked"},
        "thresholds":thresholds,"question_digest":super::questions::digest(),"context_verified":state["context_verified"],
        "thresholds_calibrated":false,"probabilities":probabilities,"reasons":reasons}),
    )
}

/// Recheck role, check and acceptance evidence after the model responds.
pub trait Evidence {
    fn is_current(&self) -> Result<bool>;
}

/// Produce a recommendation whose eligibility is bound to the final evidence snapshot.
pub fn recommend_current(
    plan: &Value,
    results: &Value,
    checks: &Value,
    state: &Value,
    classifier: &impl Classifier,
    thresholds: &Thresholds,
    evidence: &impl Evidence,
) -> Result<Value> {
    let mut report = recommend(plan, results, checks, state, classifier, thresholds)?;
    if report["automation_eligible"] == true && !evidence.is_current().unwrap_or(false) {
        report["automation_eligible"] = json!(false);
        report["recommendation"] = json!("blocked");
        report["reasons"]
            .as_array_mut()
            .context("missing reasons")?
            .push(json!({"code":ReasonCode::EvidenceChanged,"source":"gate",
                "evidence":"Role, check or acceptance evidence changed during classification."}));
    }
    Ok(report)
}

/// Render the recorded handoff, including model-localized evidence, without executing its content.
pub fn handoff_markdown(report: &Value) -> String {
    let mut text = format!("<!-- botica:review-handoff -->\n<!-- botica:review-risk base={} head={} tier={} -->\n# Review recommendation: {}\n\nHuman review required: {}. Shadow mode, no merge performed.\n\n",
        report["inputs"]["base"].as_str().unwrap_or("unknown"),
        report["inputs"]["head"].as_str().unwrap_or("unknown"),
        report["effective_tier"].as_str().unwrap_or("incomplete"),
        report["recommendation"].as_str().unwrap_or("blocked"), report["human_required"]);
    text.push_str(&format!("Inputs:\n\n    {}\n\n", report["inputs"]));
    text.push_str(&format!("## Risk and classifier\n\n    {}\n\n",
        json!({"deterministic_tier":report["deterministic_tier"],"effective_tier":report["effective_tier"],
            "raised_by_classifier":report["raised_by_classifier"],"evidence":report["risk_evidence"],
            "probabilities":report["probabilities"],"thresholds":report["thresholds"]})));
    text.push_str(&format!(
        "Decision requested: {}\n\n",
        report["decision_requested"]
            .as_str()
            .unwrap_or("resolve-evidence-blockers")
    ));
    text.push_str("## Controls reviewed\n\n");
    for role in report["review_summary"]["roles"]
        .as_array()
        .into_iter()
        .flatten()
    {
        text.push_str(&format!(
            "    {}\n",
            json!({"role":role["role"],"verdict":role["verdict"]})
        ));
    }
    text.push_str("\n## Not reviewed\n\n");
    for item in report["not_reviewed"].as_array().into_iter().flatten() {
        text.push_str(&format!("    {item}\n"));
    }
    text.push_str("\n## Localization\n\n");
    for item in report["localization"].as_array().into_iter().flatten() {
        text.push_str(&format!("    {item}\n"));
    }
    text.push('\n');
    for reason in report["reasons"].as_array().into_iter().flatten() {
        text.push_str(&format!(
            "## {}\n\nSource: {}\n\nEvidence:\n\n    {}\n\nVerify: {}\n\nResolve: {}\n\n",
            reason["code"].as_str().unwrap_or("blocked"),
            reason["source"],
            reason,
            reason["verify"].as_str().unwrap_or(""),
            reason["resolve"].as_str().unwrap_or("")
        ));
    }
    text
}
