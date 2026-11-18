use botica_core::review::classifier::{recommend, Classifier, Probabilities, Thresholds, MODEL};
use serde_json::{json, Value};
use std::cell::Cell;

struct InMemoryClassifier {
    fail: bool,
    calls: Cell<u32>,
}
impl Classifier for InMemoryClassifier {
    fn classify(&self, _: &Value) -> anyhow::Result<Probabilities> {
        self.calls.set(self.calls.get() + 1);
        anyhow::ensure!(!self.fail, "service unavailable");
        Ok(Probabilities {
            model: MODEL.into(),
            decision: 0.01,
            unverifiable: 0.01,
            contained: 0.8,
            team: 0.19,
            users: 0.01,
            confidence: 0.99,
            request_id: Some("test".into()),
        })
    }
    fn locate(&self, _: &Value, _: &str) -> anyhow::Result<f64> {
        Ok(0.8)
    }
}
fn plan(area: &str, tier: &str) -> Value {
    json!({"status":"complete","inputs":{"head":"a".repeat(40)},"mutation_required":tier!="low","required_checks":["tests"],
        "risk":{"status":"complete","effective_tier":tier,"branch":{"comparisons":[{"areas":[area]}]},"integration":{"comparisons":[]}}})
}
fn evidence() -> Value {
    json!({"inputs":{"head":"a".repeat(40)},"status":"complete","verdict":"PASS","findings":[]})
}
fn checks() -> Value {
    json!({"inputs":{"head":"a".repeat(40)},"ci":"PASS","lint":"PASS","mutation":"PASS","jobs":{"tests":"PASS"}})
}

#[test]
fn only_critical_routes_to_human_and_service_failure_never_does() {
    let adapter = InMemoryClassifier {
        fail: false,
        calls: Cell::new(0),
    };
    let critical = recommend(
        &plan("critical", "high"),
        &evidence(),
        &checks(),
        &json!({}),
        &adapter,
        &Thresholds::default(),
    )
    .unwrap();
    assert_eq!(critical["human_required"], true);
    assert_eq!(adapter.calls.get(), 0);
    let medium = recommend(
        &plan("core", "medium"),
        &evidence(),
        &checks(),
        &json!({}),
        &adapter,
        &Thresholds::default(),
    )
    .unwrap();
    assert_eq!(medium["human_required"], false);
    assert_eq!(medium["effective_tier"], "medium");
    assert_eq!(medium["automation_eligible"], false);
    let failed = InMemoryClassifier {
        fail: true,
        calls: Cell::new(0),
    };
    let report = recommend(
        &plan("periphery", "low"),
        &evidence(),
        &checks(),
        &json!({}),
        &failed,
        &Thresholds::default(),
    )
    .unwrap();
    assert_eq!(report["recommendation"], "blocked");
    assert_eq!(report["human_required"], false);
}

#[test]
fn current_low_risk_evidence_is_a_shadow_candidate_but_stale_checks_never_call_the_model() {
    let adapter = InMemoryClassifier {
        fail: false,
        calls: Cell::new(0),
    };
    let report = recommend(
        &plan("periphery", "low"),
        &evidence(),
        &checks(),
        &json!({}),
        &adapter,
        &Thresholds::default(),
    )
    .unwrap();
    assert_eq!(report["recommendation"], "automation-candidate");
    assert_eq!(report["merge_performed"], false);
    let mut stale = checks();
    stale["inputs"]["head"] = json!("b".repeat(40));
    let report = recommend(
        &plan("periphery", "low"),
        &evidence(),
        &stale,
        &json!({}),
        &adapter,
        &Thresholds::default(),
    )
    .unwrap();
    assert_eq!(report["automation_eligible"], false);
    assert_eq!(adapter.calls.get(), 1);
}
