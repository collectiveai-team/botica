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
        .map_err(|_| anyhow::anyhow!("model HTTP request failed; no response body recorded"))?;
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
