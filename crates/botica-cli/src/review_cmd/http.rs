//! Bounded model transport. Errors never include response bodies or credentials.

use anyhow::{bail, ensure, Context, Result};
use base64::Engine;
use serde_json::Value;
use std::io::Read;
use std::time::Duration;

pub(super) trait ModelTransport {
    fn post(&self, url: &str, key: &str, body: &Value) -> Result<Value>;
}

pub(super) struct LiveTransport;

impl ModelTransport for LiveTransport {
    fn post(&self, url: &str, key: &str, body: &Value) -> Result<Value> {
        post(url, key, body)
    }
}

pub(super) fn reject_credentials(value: &Value, key: &str) -> Result<()> {
    let text = value.to_string();
    let encoded = base64::engine::general_purpose::STANDARD.encode(key);
    ensure!(
        key.is_empty() || (!text.contains(key) && !text.contains(&encoded)),
        "model response contains a credential; output rejected"
    );
    Ok(())
}

pub(super) fn post(url: &str, key: &str, body: &Value) -> Result<Value> {
    ensure!(
        url.starts_with("https://") && !url.contains(['\r', '\n', '@']),
        "model endpoint must be HTTPS without userinfo"
    );
    ensure!(
        !key.is_empty() && !key.contains(['\r', '\n']),
        "model credential is unavailable or invalid"
    );
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(90)))
        .max_redirects(0)
        .build();
    let agent: ureq::Agent = config.into();
    let mut response = agent
        .post(url)
        .header("Authorization", &format!("Bearer {key}"))
        .send_json(body)
        .map_err(|error| anyhow::anyhow!("{}", transport_failure(&error)))?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow::anyhow!("model response could not be read"))?;
    ensure!(
        bytes.len() <= 2 * 1024 * 1024,
        "model response exceeds 2 MiB"
    );
    let text = std::str::from_utf8(&bytes).context("model response is not UTF-8")?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(key);
    if text.contains(key) || text.contains(&encoded) {
        bail!("model response contains a credential; output rejected");
    }
    serde_json::from_str(text).map_err(|_| anyhow::anyhow!("invalid model JSON response"))
}

/// Names the failure class and HTTP status only: provider bodies can echo prompts or secrets.
fn transport_failure(error: &ureq::Error) -> String {
    let class = match error {
        ureq::Error::StatusCode(429) => "status 429 (rate limit or quota)".to_owned(),
        ureq::Error::StatusCode(code @ (401 | 403)) => {
            format!("status {code} (credential rejected)")
        }
        ureq::Error::StatusCode(code) if *code >= 500 => format!("status {code} (provider error)"),
        ureq::Error::StatusCode(code) => format!("status {code} (request rejected)"),
        ureq::Error::Timeout(_) => "timeout (transport)".to_owned(),
        _ => "transport".to_owned(),
    };
    format!("model HTTP request failed: {class}; no response body recorded")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_failures_name_only_safe_categories() {
        for (error, expected) in [
            (
                ureq::Error::StatusCode(429),
                "status 429 (rate limit or quota)",
            ),
            (
                ureq::Error::StatusCode(401),
                "status 401 (credential rejected)",
            ),
            (ureq::Error::StatusCode(502), "status 502 (provider error)"),
            (
                ureq::Error::StatusCode(422),
                "status 422 (request rejected)",
            ),
            (ureq::Error::HostNotFound, "transport"),
        ] {
            assert_eq!(
                transport_failure(&error),
                format!("model HTTP request failed: {expected}; no response body recorded")
            );
        }
    }
}
