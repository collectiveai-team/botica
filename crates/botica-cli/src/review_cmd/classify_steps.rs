use super::ClassifyArgs;
use crate::review_cmd::{evidence, github, tier};
use botica_core::review::classifier::{Classifier, Thresholds};
use serde_json::{json, Value};

pub(super) fn roles(args: &ClassifyArgs, assessment: &tier::Assessment, plan: &Value) -> Value {
    let mut results = evidence::collect(assessment, plan, &args.results)
        .unwrap_or_else(|_| json!({"status":"incomplete","verdict":"INCONCLUSIVE","inputs":null}));
    let provenance = args.github_pr.and_then(|pr| {
        github::verify_roles(
            args.repository.as_deref().unwrap_or(""),
            pr,
            assessment,
            plan,
            &results,
        )
        .ok()
    });
    results["producer_verified"] = json!(provenance.is_some());
    results["provenance"] = provenance.unwrap_or(Value::Null);
    results
}

pub(super) fn checks(args: &ClassifyArgs, plan: &Value) -> Value {
    if let Some(pr) = args.github_pr {
        return github::checks(args.repository.as_deref().unwrap_or(""), pr, plan)
            .unwrap_or_else(|_| json!({"producer_verified":false,"status":"unavailable"}));
    }
    let mut checks = args
        .checks
        .as_deref()
        .and_then(|path| evidence::read_json(path).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    checks["producer_verified"] = json!(false);
    checks
}

pub(super) fn context_current(args: &ClassifyArgs, plan: &Value) -> bool {
    args.github_pr.is_some_and(|pr| {
        github::context_matches(args.repository.as_deref().unwrap_or(""), pr, plan).unwrap_or(false)
    })
}

pub(super) fn thresholds(assessment: &tier::Assessment) -> Thresholds {
    assessment
        .review
        .as_ref()
        .map(|review| Thresholds {
            maximum_question: review.classifier.question_max,
            minimum_contained_team: review.classifier.impact_min,
            minimum_confidence: review.classifier.min_confidence,
        })
        .unwrap_or_default()
}

pub(super) fn invalidate_changed_context(
    report: &mut Value,
    args: &ClassifyArgs,
    plan: &Value,
    was_verified: bool,
) {
    if was_verified && !context_current(args, plan) {
        report["automation_eligible"] = json!(false);
        if report["human_required"] != true {
            report["recommendation"] = json!("blocked");
        }
        report["reasons"].as_array_mut().unwrap().push(json!({"code":"context-changed",
            "source":"gate","evidence":"The GitHub acceptance context changed during classification."}));
    }
}

fn localization_requests(plan: &Value, report: &Value) -> Vec<(String, String)> {
    let paths: std::collections::BTreeSet<_> = plan["risk"]["branch"]["comparisons"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|comparison| {
            comparison["files"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
        })
        .collect();
    report["reasons"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|reason| reason["code"].as_str())
        .filter(|code| matches!(*code, "decision" | "unverifiable"))
        .flat_map(|code| {
            paths
                .iter()
                .map(move |path| (code.to_owned(), (*path).to_owned()))
        })
        .collect()
}

fn localize_file(
    assessment: &tier::Assessment,
    context: &Value,
    adapter: &impl Classifier,
    code: &str,
    path: &str,
) -> Value {
    let content = assessment.git.read_limited(
        &["show", &format!("{}:{path}", assessment.git.head)],
        16_000,
    );
    let Ok(content) = content else {
        return json!({"reason":code,"path":path,"status":"unavailable"});
    };
    let state = json!({"context":context,"path":path,"file":String::from_utf8_lossy(&content)});
    let probability = adapter.locate(&state, code).ok();
    json!({"reason":code,"path":path,"probability":probability,
        "status":if probability.is_some() {"measured"} else {"unavailable"}})
}

pub(super) fn localization(
    assessment: &tier::Assessment,
    plan: &Value,
    adapter: &impl Classifier,
    report: &Value,
) -> Vec<Value> {
    localization_requests(plan, report)
        .iter()
        .map(|(code, path)| {
            localize_file(assessment, &plan["untrusted_context"], adapter, code, path)
        })
        .collect()
}

fn decision_requested(report: &Value) -> &'static str {
    if report["human_required"] == true {
        "critical-owner-review"
    } else if report["automation_eligible"] == true {
        "none-shadow-only"
    } else {
        "resolve-evidence-blockers"
    }
}

pub(super) fn finish(
    mut report: Value,
    localization: Vec<Value>,
    not_reviewed: Vec<Value>,
    results: Value,
    checks: Value,
) -> Value {
    report["localization"] = json!(localization);
    report["not_reviewed"] = json!(not_reviewed);
    report["decision_requested"] = json!(decision_requested(&report));
    report["review_summary"] = results;
    report["checks"] = checks;
    super::handoff(&mut report);
    report
}
