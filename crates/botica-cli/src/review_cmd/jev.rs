//! Jev System One adapter, with fixed questions and impact option order.

use super::http;
use anyhow::{ensure, Context, Result};
use botica_core::review::classifier::{Classifier, Probabilities, MODEL};
use serde_json::{json, Value};

pub(super) struct Jev<T = http::LiveTransport> {
    url: String,
    key: String,
    transport: T,
}

impl Jev<http::LiveTransport> {
    pub(super) fn configured() -> Self {
        let base = std::env::var("TYPESAFE_AI_BASE_URL")
            .unwrap_or_else(|_| "https://api.typesafe.ai".into());
        Self {
            url: format!("{}/v1/systemone", base.trim_end_matches('/')),
            key: std::env::var("TYPESAFE_AI_API_KEY").unwrap_or_default(),
            transport: http::LiveTransport,
        }
    }
}

impl<T: http::ModelTransport> Jev<T> {
    fn ask(&self, state: &Value, questions: Value) -> Result<Value> {
        let response = self.transport.post(
            &self.url,
            &self.key,
            &json!({"model":MODEL,"state":state,"questions":questions}),
        )?;
        http::reject_credentials(&response, &self.key)?;
        ensure!(
            response["model"] == MODEL,
            "Jev returned an unexpected model"
        );
        Ok(response)
    }
}

fn question(name: &str) -> Result<Value> {
    let (instructions, yes, no) = match name {
        "decision" => ("Does the change choose something the linked issue or description did not decide?",
            "Chooses user-visible behavior, a dependency, a convention or a tradeoff between alternatives not fixed by the issue.",
            "Implements a choice already fixed in the linked issue or description."),
        "unverifiable" => ("Does part of the change need direct observation beyond the supplied tests and checks?",
            "Changes user-facing text, prompts, CLI output, production settings or performance without direct verification.",
            "The supplied tests and deterministic checks verify the behavior."),
        _ => anyhow::bail!("unknown classifier question"),
    };
    Ok(json!({"type":"noul","instructions":instructions,"criteria":{"true":yes,"false":no}}))
}

fn probability(value: &Value) -> Result<f64> {
    let probability = value.as_f64().context("missing Jev probability")?;
    ensure!(
        probability.is_finite() && (0.0..=1.0).contains(&probability),
        "invalid Jev probability"
    );
    Ok(probability)
}

fn noul(answer: &Value) -> Result<f64> {
    ensure!(answer["type"] == "noul", "unexpected Jev answer type");
    probability(&answer["noul"])
}

impl<T: http::ModelTransport> Classifier for Jev<T> {
    fn classify(&self, state: &Value) -> Result<Probabilities> {
        let response = self.ask(state, json!({
            "decision":question("decision")?,
            "unverifiable":question("unverifiable")?,
            "impact_if_wrong":{"type":"choice","instructions":"If this change is wrong despite passing its supplied checks, who notices first?",
                "criteria":{"contained":"A developer in this repository: tests, internal docs or scripts.",
                    "team":"The team: CI, tooling or recipes before release.",
                    "users":"A botica user: published rules, CLI or installed recipe templates."}}
        }))?;
        let answer = &response["answers"];
        ensure!(
            answer["impact_if_wrong"]["type"] == "choice",
            "unexpected Jev impact type"
        );
        let impact = &answer["impact_if_wrong"]["probabilities"];
        ensure!(
            impact.as_object().is_some_and(|options| options.len() == 3),
            "invalid impact options"
        );
        let result = Probabilities {
            model: MODEL.into(),
            decision: noul(&answer["decision"])?,
            unverifiable: noul(&answer["unverifiable"])?,
            contained: probability(&impact["contained"])?,
            team: probability(&impact["team"])?,
            users: probability(&impact["users"])?,
            confidence: probability(&answer["impact_if_wrong"]["confidence"])?,
            request_id: response["request_id"].as_str().map(str::to_owned),
        };
        result.validate()?;
        Ok(result)
    }

    fn locate(&self, state: &Value, reason: &str) -> Result<f64> {
        let response = self.ask(state, json!({reason:question(reason)?}))?;
        noul(&response["answers"][reason])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InMemoryTransport(Value);
    impl http::ModelTransport for InMemoryTransport {
        fn post(&self, _: &str, _: &str, body: &Value) -> Result<Value> {
            ensure!(body["model"] == MODEL, "model was not pinned");
            let options = body["questions"]["impact_if_wrong"]["criteria"].as_object();
            if let Some(options) = options {
                ensure!(
                    options.keys().map(String::as_str).collect::<Vec<_>>()
                        == ["contained", "team", "users"],
                    "impact options changed"
                );
            }
            Ok(self.0.clone())
        }
    }

    fn adapter(answer: Value) -> Jev<InMemoryTransport> {
        Jev {
            url: "https://service.invalid/v1/systemone".into(),
            key: "synthetic-key".into(),
            transport: InMemoryTransport(answer),
        }
    }

    #[test]
    fn jev_adapter_reads_real_api_shape_and_rejects_wrong_models_or_probabilities() {
        let response = json!({"model":MODEL,"answers":{
            "decision":{"type":"noul","noul":0.01},"unverifiable":{"type":"noul","noul":0.02},
            "impact_if_wrong":{"type":"choice","choice":"contained","confidence":0.98,
                "probabilities":{"contained":0.8,"team":0.19,"users":0.01}}
        }});
        let result = adapter(response.clone())
            .classify(&json!({"diff":"A real change"}))
            .unwrap();
        assert_eq!(result.contained, 0.8);
        assert_eq!(result.decision, 0.01);
        let mut invalid = response.clone();
        invalid["model"] = json!("jev-latest");
        assert!(adapter(invalid).classify(&json!({})).is_err());
        let mut invalid = response;
        invalid["answers"]["decision"]["noul"] = json!(1.2);
        assert!(adapter(invalid).classify(&json!({})).is_err());
    }
}
