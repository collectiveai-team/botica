//! Jev System One adapter, with fixed questions and impact option order.

use super::http;
use anyhow::{ensure, Context, Result};
use botica_core::review::classifier::{
    Classifier, ContextLimit, Probabilities, INPUT_TOKEN_LIMIT, MODEL, REQUEST_BYTE_LIMIT,
};
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
        let request = json!({"model":MODEL,"state":state,"questions":questions});
        ensure!(
            serde_json::to_vec(&request)?.len() <= REQUEST_BYTE_LIMIT,
            "model request exceeds the local byte limit"
        );
        let response = self.transport.post(&self.url, &self.key, &request)?;
        http::reject_credentials(&response, &self.key)?;
        ensure!(
            response["model"] == MODEL,
            "Jev returned an unexpected model"
        );
        let input_tokens = response["usage"]["input_tokens"]
            .as_u64()
            .context("Jev did not report a valid input-token count")?;
        if input_tokens > INPUT_TOKEN_LIMIT {
            return Err(ContextLimit { input_tokens }.into());
        }
        Ok(response)
    }
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
        let response = self.ask(state, botica_core::review::questions::all())?;
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
        let response = self.ask(
            state,
            json!({reason:botica_core::review::questions::noul(reason)?}),
        )?;
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
        let response = json!({"model":MODEL,"usage":{"input_tokens":100},"answers":{
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

    #[test]
    fn jev_accepts_only_measured_usage_within_the_token_budget_for_both_operations() {
        let response = json!({"model":MODEL,"usage":{"input_tokens":32_000},"answers":{
            "decision":{"type":"noul","noul":0.01},"unverifiable":{"type":"noul","noul":0.02},
            "impact_if_wrong":{"type":"choice","confidence":0.98,
                "probabilities":{"contained":0.8,"team":0.19,"users":0.01}}
        }});
        for tokens in [31_999, 32_000] {
            let mut answer = response.clone();
            answer["usage"]["input_tokens"] = json!(tokens);
            let adapter = adapter(answer);
            assert!(adapter
                .classify(&json!({"diff":"reviewable ".repeat(4_000)}))
                .is_ok());
            assert!(adapter.locate(&json!({}), "decision").is_ok());
            assert!(adapter
                .classify(&json!({"diff":"x".repeat(REQUEST_BYTE_LIMIT)}))
                .is_err());
        }
        for usage in [
            json!(32_001),
            Value::Null,
            json!(-1),
            json!(1.5),
            json!("32000"),
        ] {
            let mut answer = response.clone();
            answer["usage"]["input_tokens"] = usage;
            let adapter = adapter(answer);
            assert!(adapter.classify(&json!({})).is_err());
            assert!(adapter.locate(&json!({}), "decision").is_err());
        }
        let mut missing = response;
        missing.as_object_mut().unwrap().remove("usage");
        assert!(adapter(missing).classify(&json!({})).is_err());
    }
}
