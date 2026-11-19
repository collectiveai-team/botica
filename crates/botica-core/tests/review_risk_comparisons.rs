//! Role selection and critical routing read both the branch and the integration comparisons.

use botica_core::review::classifier::{recommend, Classifier, Probabilities, Thresholds, MODEL};
use botica_core::review::roles::RoleDefinition;
use serde_json::{json, Value};

struct Unused;

impl Classifier for Unused {
    fn classify(&self, _: &Value) -> anyhow::Result<Probabilities> {
        Ok(Probabilities {
            model: MODEL.into(),
            decision: 0.01,
            unverifiable: 0.01,
            contained: 0.8,
            team: 0.19,
            users: 0.01,
            confidence: 0.99,
            request_id: None,
        })
    }
    fn locate(&self, _: &Value, _: &str) -> anyhow::Result<f64> {
        Ok(0.1)
    }
}

fn risk(branch: Value, integration: Value) -> Value {
    json!({"status":"complete","effective_tier":"medium",
        "branch":{"status":"complete","tier":"medium","comparisons":[branch]},
        "integration":{"status":"complete","tier":"medium","comparisons":[integration]}})
}

fn comparison(file: &str, area: &str) -> Value {
    json!({"files":[file],"areas":[area],"signals":[],"bounds":null})
}

#[test]
fn a_role_path_that_only_the_integration_result_touches_selects_the_role() {
    let role = RoleDefinition::parse(
        "security",
        "---\nrole: security\napplies:\n  paths: [\"src/auth/**\"]\nstages: [ci]\nblocking: P1\nmax-findings: 10\n---\n",
    )
    .unwrap();
    for (branch, integration, selected) in [
        ("docs/a.md", "src/auth/login.rs", true),
        ("src/auth/login.rs", "docs/a.md", true),
        ("docs/a.md", "docs/b.md", false),
    ] {
        let risk = risk(comparison(branch, "core"), comparison(integration, "core"));
        let found = role.selection(&risk, "ci").unwrap();
        assert_eq!(
            found["selected"], selected,
            "{branch} {integration}: {found}"
        );
    }
}

#[test]
fn a_critical_area_that_only_the_integration_result_touches_requires_a_person() {
    for (branch, integration, human) in [
        ("core", "critical", true),
        ("critical", "core", true),
        ("core", "periphery", false),
    ] {
        let inputs = json!({"head":"a".repeat(40)});
        let plan = json!({"status":"complete","inputs":inputs,"mutation_required":true,
            "required_checks":["tests"],
            "risk":risk(comparison("a.rs", branch), comparison("a.rs", integration))});
        let report = recommend(
            &plan,
            &json!({"inputs":inputs,"producer_verified":true,"status":"complete","verdict":"PASS","findings":[]}),
            &json!({"inputs":inputs,"producer_verified":true,"ci":"PASS","lint":"PASS","mutation":"PASS","jobs":{"tests":"PASS"}}),
            &json!({"context_verified":true,"change_context_complete":true,
                "context":{"title":"t","description":"d","acceptance_criteria":["a"]}}),
            &Unused,
            &Thresholds::default(),
        )
        .unwrap();
        assert_eq!(report["human_required"], human, "{branch} {integration}");
    }
}
