//! The decision content sent to the classifier, separate from gate provenance.

use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// One comparison's literal paths retained in classifier context.
pub struct DiffSelection {
    pub from: String,
    pub to: String,
    pub paths: Vec<String>,
}

/// Select complete text evidence for the classifier without changing role-review inputs.
pub fn diff_selections(plan: &Value) -> Result<Vec<DiffSelection>> {
    let mut pairs = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for comparison in comparisons(plan) {
        let pair = (
            comparison["from"]
                .as_str()
                .context("missing comparison base")?
                .to_owned(),
            comparison["to"]
                .as_str()
                .context("missing comparison result")?
                .to_owned(),
        );
        let paths = pairs.entry(pair).or_default();
        for file in comparison["evidence"]
            .as_array()
            .context("missing change evidence")?
        {
            if file["generated"] == true {
                continue;
            }
            ensure!(
                file["kind"] == "text",
                "non-text change context is unavailable"
            );
            for path in file["paths"].as_array().context("missing change paths")? {
                paths.insert(path.as_str().context("non-text change path")?.to_owned());
            }
        }
    }
    Ok(pairs
        .into_iter()
        .map(|((from, to), paths)| DiffSelection {
            from,
            to,
            paths: paths.into_iter().collect(),
        })
        .collect())
}

fn project(value: &Value, fields: &[&str]) -> Value {
    Value::Object(
        fields
            .iter()
            .filter_map(|field| {
                value
                    .get(*field)
                    .map(|value| ((*field).to_owned(), value.clone()))
            })
            .collect(),
    )
}

fn comparisons(plan: &Value) -> impl Iterator<Item = &Value> {
    super::risk_comparisons(&plan["risk"])
}

fn distinct(values: impl Iterator<Item = Value>) -> Vec<Value> {
    values
        .map(|value| (value.to_string(), value))
        .collect::<BTreeMap<_, _>>()
        .into_values()
        .collect()
}

fn files(plan: &Value) -> Vec<Value> {
    distinct(
        comparisons(plan)
            .flat_map(|comparison| comparison["evidence"].as_array().into_iter().flatten())
            .map(|file| {
                let areas: BTreeSet<_> = file["matches"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|area| area["area"].as_str())
                    .collect();
                json!({"paths":file["paths"],"areas":areas,"generated":file["generated"],
                "kind":file["kind"],"status":file["status"]})
            }),
    )
}

/// Build model context while the caller retains the original evidence for gate validation.
pub fn build(plan: &Value, reviews: &Value, checks: &Value, diff: Value, verified: bool) -> Value {
    let roles: Vec<_> = reviews["roles"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|role| project(role, &["role", "verdict", "not_checked"]))
        .collect();
    let signals = distinct(
        comparisons(plan)
            .flat_map(|comparison| {
                comparison["signal_evidence"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .cloned(),
    );
    let unknown = distinct(
        comparisons(plan)
            .flat_map(|comparison| {
                comparison["detector_evidence"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .filter(|detector| detector["status"] != "supported")
            .cloned(),
    );
    json!({
        "context":project(&plan["untrusted_context"], &["title", "description", "issues", "acceptance_criteria"]),
        "files":files(plan),"signals":signals,"unknown_detectors":unknown,
        "summary":{"deterministic_tier":plan["risk"]["effective_tier"],
            "selected_roles":plan["selected_roles"],"roles":roles,
            "review_status":reviews["status"],"review_verdict":reviews["verdict"],
            "findings":reviews["findings"],"required_checks":plan["required_checks"],
            "checks":project(checks, &["ci", "lint", "mutation", "jobs"])},
        "context_verified":verified,"change_context_complete":diff.is_array(),"diff":diff
    })
}
