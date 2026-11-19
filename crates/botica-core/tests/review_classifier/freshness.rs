use super::*;
use botica_core::review::classifier::{recommend_current, Evidence};
use std::cell::RefCell;

struct InMemoryEvidence {
    initial: Value,
    current: RefCell<Option<Value>>,
    reads: Cell<u32>,
}

impl Evidence for InMemoryEvidence {
    fn is_current(&self) -> anyhow::Result<bool> {
        self.reads.set(self.reads.get() + 1);
        let current = self.current.borrow();
        let current = current
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("evidence unavailable"))?;
        Ok(*current == self.initial)
    }
}

struct DelayedClassifier<'a> {
    evidence: &'a InMemoryEvidence,
    after_request: Option<Value>,
    calls: Cell<u32>,
}

impl Classifier for DelayedClassifier<'_> {
    fn classify(&self, state: &Value) -> anyhow::Result<Probabilities> {
        self.calls.set(self.calls.get() + 1);
        // The response boundary advances GitHub state without a timing-dependent sleep.
        self.evidence.current.replace(self.after_request.clone());
        InMemoryClassifier {
            fail: false,
            calls: Cell::new(0),
        }
        .classify(state)
    }

    fn locate(&self, _: &Value, _: &str) -> anyhow::Result<f64> {
        anyhow::bail!("an eligible response must not need localization")
    }
}

#[test]
fn a_delayed_eligible_response_cannot_outlive_checks_roles_context_or_evidence_availability() {
    let initial = json!({"check_status":"success","role_run":77,"context_digest":"before"});
    for change in [
        "pending-check",
        "failed-check",
        "role-rerun",
        "context-edit",
        "unavailable",
        "unchanged",
    ] {
        let mut after_request = Some(initial.clone());
        match change {
            "pending-check" => {
                after_request.as_mut().unwrap()["check_status"] = json!("in_progress")
            }
            "failed-check" => after_request.as_mut().unwrap()["check_status"] = json!("failure"),
            "role-rerun" => after_request.as_mut().unwrap()["role_run"] = json!(78),
            "context-edit" => after_request.as_mut().unwrap()["context_digest"] = json!("after"),
            "unavailable" => after_request = None,
            _ => {}
        }
        let store = InMemoryEvidence {
            initial: initial.clone(),
            current: RefCell::new(Some(initial.clone())),
            reads: Cell::new(0),
        };
        let classifier = DelayedClassifier {
            evidence: &store,
            after_request,
            calls: Cell::new(0),
        };
        let report = recommend_current(
            &plan("periphery", "low"),
            &evidence(),
            &checks(),
            &state(),
            &classifier,
            &Thresholds::default(),
            &store,
        )
        .unwrap();
        assert_eq!(classifier.calls.get(), 1);
        assert_eq!(
            report["automation_eligible"],
            change == "unchanged",
            "{change}: {report}"
        );
        assert_eq!(report["human_required"], false);
        assert_eq!(report["merge_performed"], false);
        assert_eq!(store.reads.get(), 1);
        if change != "unchanged" {
            assert_eq!(report["recommendation"], "blocked");
            assert!(report["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|reason| reason["code"] == "evidence-changed"));
        }
    }
}

#[test]
fn missing_risk_or_change_context_cannot_start_the_model_or_fall_back_to_low() {
    let classifier = InMemoryClassifier {
        fail: false,
        calls: Cell::new(0),
    };
    for missing in ["risk", "change-context"] {
        let mut plan = plan("periphery", "low");
        let mut state = state();
        if missing == "risk" {
            plan["risk"]["effective_tier"] = Value::Null;
        } else {
            state
                .as_object_mut()
                .unwrap()
                .remove("change_context_complete");
        }
        let report = recommend(
            &plan,
            &evidence(),
            &checks(),
            &state,
            &classifier,
            &Thresholds::default(),
        )
        .unwrap();
        assert_eq!(report["automation_eligible"], false);
        assert_eq!(report["human_required"], false);
        assert_eq!(classifier.calls.get(), 0);
    }
}
