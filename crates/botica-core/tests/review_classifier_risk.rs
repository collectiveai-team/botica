use botica_core::review::classifier::{recommend, Classifier, Probabilities, Thresholds, MODEL};
use serde_json::{json, Value};

struct SemanticRisk(bool);

impl Classifier for SemanticRisk {
    fn classify(&self, _: &Value) -> anyhow::Result<Probabilities> {
        anyhow::ensure!(self.0, "service unavailable");
        Ok(Probabilities {
            model: MODEL.into(),
            decision: 0.9,
            unverifiable: 0.01,
            contained: 0.8,
            team: 0.19,
            users: 0.01,
            confidence: 0.99,
            request_id: None,
        })
    }
    fn locate(&self, _: &Value, _: &str) -> anyhow::Result<f64> {
        Ok(0.9)
    }
}

#[test]
fn semantic_risk_can_raise_the_floor_without_changing_critical_only_human_routing() {
    for (floor, service, effective) in [
        ("low", true, "medium"),
        ("medium", true, "high"),
        ("high", true, "high"),
        ("low", false, "low"),
    ] {
        let inputs = json!({"head":"a".repeat(40)});
        let plan = json!({"status":"complete","inputs":inputs,"mutation_required":false,
            "required_checks":["tests"],"risk":{"status":"complete","effective_tier":floor,
                "branch":{"comparisons":[{"areas":["core"]}]},"integration":{"comparisons":[]}}});
        let results = json!({"inputs":inputs,"producer_verified":true,"status":"complete","verdict":"PASS","findings":[]});
        let checks = json!({"inputs":inputs,"producer_verified":true,"ci":"PASS","lint":"PASS","jobs":{"tests":"PASS"}});
        let state = json!({"context_verified":true,"change_context_complete":true,
            "context":{"title":"Answer","description":"Implement the decided answer","acceptance_criteria":["42"]}});
        let report = recommend(
            &plan,
            &results,
            &checks,
            &state,
            &SemanticRisk(service),
            &Thresholds::default(),
        )
        .unwrap();
        assert_eq!(report["deterministic_tier"], floor);
        assert_eq!(report["effective_tier"], effective);
        assert_eq!(report["raised_by_classifier"], service && floor != "high");
        assert_eq!(report["human_required"], false);
        assert_eq!(report["automation_eligible"], false);
    }
}
