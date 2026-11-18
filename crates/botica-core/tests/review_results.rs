use botica_core::review::execution::{execute, ReviewResults, Reviewer};
use botica_core::review::results::validate;
use serde_json::{json, Value};
use std::cell::RefCell;

struct InMemoryReviewer;
impl Reviewer for InMemoryReviewer {
    fn review(&self, role: &str, prior: &[Value]) -> anyhow::Result<Value> {
        if role == "refute" {
            assert_eq!(prior.len(), 1);
            assert_eq!(prior[0]["role"], "security");
        } else {
            assert!(prior.is_empty());
        }
        Ok(result(role))
    }
}

#[derive(Default)]
struct InMemoryResults(RefCell<Vec<Value>>);
impl ReviewResults for InMemoryResults {
    fn save(&mut self, _: &str, result: &Value) -> anyhow::Result<()> {
        self.0.get_mut().push(result.clone());
        Ok(())
    }
}

#[test]
fn execution_keeps_roles_independent_and_refutes_last_through_the_public_ports() {
    let mut destination = InMemoryResults::default();
    let report = execute(&plan(), &InMemoryReviewer, &mut destination).unwrap();
    assert_eq!(report["verdict"], "PASS");
    assert_eq!(destination.0.borrow().len(), 2);
}

fn plan() -> Value {
    json!({"status":"complete", "inputs":{"head":"a".repeat(40), "base":"b".repeat(40), "roles_digest":"c".repeat(64)},
        "selected_roles":["security","refute"], "roles":[
            {"role":"security","blocking":"P1","max_findings":10},
            {"role":"refute","blocking":"P1","max_findings":50}]})
}

fn result(role: &str) -> Value {
    json!({"role":role,"head":"a".repeat(40),"base":"b".repeat(40),"inputs":plan()["inputs"],
        "verdict":"PASS","findings":[],"checked":["change"],"not_checked":[],"commands":[]})
}

#[test]
fn validated_complete_role_evidence_passes_but_missing_or_stale_evidence_cannot() {
    let reports = vec![result("security"), result("refute")];
    assert_eq!(validate(&plan(), &reports).unwrap()["verdict"], "PASS");
    assert!(validate(&plan(), &reports[..1]).is_err());
    let mut stale = reports;
    stale[0]["inputs"]["roles_digest"] = json!("d".repeat(64));
    assert!(validate(&plan(), &stale).is_err());
}

#[test]
fn a_model_pass_cannot_hide_a_blocking_finding_or_let_refute_rewrite_it() {
    let finding = json!({"id":"security-1","severity":"P1","path":"src/auth.rs","line":3,
        "title":"Authorization bypass","evidence":"The public function returns true.",
        "reproduced":"not-tried","how_to_verify":["Inspect authorize"],"fix":"Check permission"});
    let mut reports = vec![result("security"), result("refute")];
    reports[0]["findings"] = json!([finding]);
    assert!(validate(&plan(), &reports).is_err());
    reports[1]["findings"] = reports[0]["findings"].clone();
    reports[1]["findings"][0]["refutation"] = json!("confirmed");
    assert_eq!(validate(&plan(), &reports).unwrap()["verdict"], "FAIL");
    reports[1]["findings"][0]["refutation"] = json!("not-reproduced");
    assert_eq!(validate(&plan(), &reports).unwrap()["verdict"], "FAIL");
    reports[1]["findings"][0]["refutation"] = json!("unreachable");
    assert!(validate(&plan(), &reports).is_err());
    reports[1]["findings"][0]["refutation"] = json!("confirmed");
    reports[1]["findings"][0]["severity"] = json!("P3");
    assert!(validate(&plan(), &reports).is_err());
}

#[test]
fn duplicate_cannot_transfer_a_blocking_finding_to_a_weaker_role_threshold() {
    let mut plan = plan();
    plan["selected_roles"] = json!(["security", "tests", "refute"]);
    plan["roles"]
        .as_array_mut()
        .unwrap()
        .push(json!({"role":"tests","blocking":"P0","max_findings":10}));
    let finding = json!({"id":"security-1","severity":"P1","path":"src/auth.rs","line":3,
        "title":"Authorization bypass","evidence":"The public function returns true.",
        "reproduced":"not-tried","how_to_verify":["Inspect authorize"],"fix":"Check permission"});
    let mut other = finding.clone();
    other["id"] = json!("tests-1");
    let mut reports = vec![result("security"), result("tests"), result("refute")];
    reports[0]["findings"] = json!([finding]);
    reports[1]["findings"] = json!([other]);
    reports[2]["findings"] = json!([finding, other]);
    reports[2]["findings"][0]["refutation"] = json!("duplicate");
    reports[2]["findings"][0]["duplicate_of"] = json!("tests-1");
    reports[2]["findings"][0]["refutation_evidence"] = json!("Same reported behavior");
    reports[2]["findings"][0]["refutation_references"] = json!([{"path":"src/auth.rs",
        "line":3,"revision":plan["inputs"]["head"],"quote":"return true"}]);
    assert!(validate(&plan, &reports)
        .unwrap_err()
        .to_string()
        .contains("blocking effect"));
    plan["roles"][2]["blocking"] = json!("P1");
    assert_eq!(validate(&plan, &reports).unwrap()["verdict"], "FAIL");
}
