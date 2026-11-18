//! Evaluation rejects structurally incomplete observations through the public CLI.

mod common;
use serde_json::{json, Value};

#[test]
fn incomplete_or_mismatched_observations_cannot_be_measured_as_candidates() {
    let dir = common::project_in("review-evaluate", "observations", None);
    let case = json!({"id":"synthetic-1","source":"https://github.com/owner/repo/pull/1",
        "base":"b".repeat(40),"head":"a".repeat(40),"label":true,"label_source":"confirmed",
        "label_evidence":"Synthetic test fixture, not evaluation data","floor":null,"critical":null,
        "checks_passed":null,"reviews_passed":null,"context_complete":null,
        "inputs_verified":false,"probabilities":null,"question_digest":null});
    common::write(&dir, ".tmp/cases.jsonl", &format!("{case}\n"));
    let inputs = json!({"base":"b".repeat(40),"head":"a".repeat(40)});
    let observation = json!({"schema_version":1,"mode":"shadow","merge_performed":false,
        "inputs":inputs,"effective_tier":"low","human_required":false,"reasons":[],"context_verified":true,
        "checks":{"inputs":inputs,"producer_verified":true,"ci":"PASS","lint":"PASS"},
        "review_summary":{"inputs":inputs,"producer_verified":true,"status":"complete","verdict":"PASS"},
        "question_digest":botica_core::review::questions::digest(),
        "probabilities":{"model":botica_core::review::classifier::MODEL,"decision":0.01,
            "unverifiable":0.01,"contained":0.8,"team":0.19,"users":0.01,"confidence":0.99,"request_id":null}});
    for invalid in [
        "missing-reasons",
        "wrong-reasons",
        "stale-checks",
        "stale-reviews",
        "blocked-checks",
        "byte-limit",
        "valid",
    ] {
        let mut report = observation.clone();
        match invalid {
            "missing-reasons" => {
                report.as_object_mut().unwrap().remove("reasons");
            }
            "wrong-reasons" => report["reasons"] = json!({}),
            "stale-checks" => report["checks"]["inputs"]["head"] = json!("c".repeat(40)),
            "stale-reviews" => report["review_summary"]["inputs"]["head"] = json!("c".repeat(40)),
            "blocked-checks" => {
                report["reasons"] = json!([{"code":"required-checks","source":"gate"}])
            }
            "byte-limit" => {
                report["reasons"] = json!([{"code":"context-byte-limit","source":"gate"}]);
                report["probabilities"] = Value::Null;
            }
            _ => {}
        }
        common::write(&dir, ".tmp/observation.json", &report.to_string());
        let output = common::command(
            &dir,
            &[
                "review",
                "evaluate",
                "--dataset",
                ".tmp/cases.jsonl",
                "--observation",
                ".tmp/observation.json",
            ],
        );
        if matches!(invalid, "valid" | "blocked-checks" | "byte-limit") {
            assert!(output.status.success(), "{}", common::stderr(&output));
            let measured: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(measured["evaluated"], usize::from(invalid == "valid"));
            assert_eq!(
                measured["automation_candidates"],
                usize::from(invalid == "valid")
            );
            assert_eq!(measured["unmeasured"], usize::from(invalid != "valid"));
            assert_eq!(measured["ready_for_activation"], false);
            if invalid == "byte-limit" {
                let mut expected: botica_core::review::evaluation::EvaluationCase =
                    serde_json::from_value(case.clone()).unwrap();
                expected.floor = Some("low".to_owned());
                expected.critical = Some(false);
                expected.checks_passed = Some(true);
                expected.reviews_passed = Some(true);
                expected.context_complete = Some(false);
                expected.question_digest = Some(botica_core::review::questions::digest());
                let digest = blake3::hash(&serde_json::to_vec(&vec![expected]).unwrap());
                assert_eq!(measured["dataset_digest"], digest.to_hex().to_string());
            }
        } else {
            assert_eq!(
                output.status.code(),
                Some(2),
                "{invalid}: {}",
                common::stderr(&output)
            );
        }
    }
}
