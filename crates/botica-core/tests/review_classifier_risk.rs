use botica_core::review::classifier::{recommend, Classifier, Probabilities, Thresholds, MODEL};
use serde_json::{json, Value};

/// A classifier whose answer crosses exactly one threshold of `Thresholds::default()`, or fails.
struct SemanticRisk(Option<&'static str>);

impl Classifier for SemanticRisk {
    fn classify(&self, _: &Value) -> anyhow::Result<Probabilities> {
        let crossed = self
            .0
            .ok_or_else(|| anyhow::anyhow!("service unavailable"))?;
        let mut answer = Probabilities {
            model: MODEL.into(),
            decision: 0.01,
            unverifiable: 0.01,
            contained: 0.8,
            team: 0.19,
            users: 0.01,
            confidence: 0.99,
            request_id: None,
        };
        match crossed {
            "decision" => answer.decision = 0.9,
            "unverifiable" => answer.unverifiable = 0.9,
            "impact-users" => (answer.contained, answer.team, answer.users) = (0.05, 0.05, 0.9),
            "low-confidence" => answer.confidence = 0.1,
            _ => unreachable!(),
        }
        Ok(answer)
    }
    fn locate(&self, _: &Value, _: &str) -> anyhow::Result<f64> {
        Ok(0.9)
    }
}

#[test]
fn semantic_risk_can_raise_the_floor_without_changing_critical_only_human_routing() {
    for reason in ["decision", "unverifiable", "impact-users", "low-confidence"] {
        let raises = reason != "low-confidence";
        for (floor, service, raised_to) in [
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
                &SemanticRisk(service.then_some(reason)),
                &Thresholds::default(),
            )
            .unwrap();
            let effective = if raises { raised_to } else { floor };
            assert_eq!(report["deterministic_tier"], floor, "{reason} {floor}");
            assert_eq!(report["effective_tier"], effective, "{reason} {floor}");
            assert_eq!(
                report["raised_by_classifier"],
                raises && service && floor != "high",
                "{reason} {floor}"
            );
            if service {
                assert!(
                    report["reasons"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|found| found["code"] == reason && found["source"] == "Jev"),
                    "{reason}: {}",
                    report["reasons"]
                );
            }
            assert_eq!(report["human_required"], false);
            assert_eq!(report["automation_eligible"], false);
        }
    }
}
