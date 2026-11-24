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

/// A failure that a later identical request can succeed on: rate limit, provider error, timeout
/// or a cut-off connection. Credential and request rejections are final.
#[derive(Debug)]
struct Transient(String);

impl std::fmt::Display for Transient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Transient {}

const ATTEMPTS: u32 = 3;

/// Posts once, then repeats the same request after a transient failure, at most `ATTEMPTS` times.
pub(super) fn post(url: &str, key: &str, body: &Value) -> Result<Value> {
    retry(ATTEMPTS, |attempt| {
        if attempt > 0 {
            std::thread::sleep(Duration::from_secs(10 * u64::from(attempt)));
        }
        post_once(url, key, body)
    })
}

fn retry(attempts: u32, mut send: impl FnMut(u32) -> Result<Value>) -> Result<Value> {
    let mut attempt = 0;
    loop {
        match send(attempt) {
            Err(error) if attempt + 1 < attempts && error.is::<Transient>() => attempt += 1,
            Err(error) if error.is::<Transient>() => {
                return Err(error.context(format!("after {attempts} attempts")));
            }
            other => return other,
        }
    }
}

fn post_once(url: &str, key: &str, body: &Value) -> Result<Value> {
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
        .map_err(|error| {
            let message = transport_failure(&error);
            if is_transient(&error) {
                anyhow::Error::new(Transient(message))
            } else {
                anyhow::anyhow!(message)
            }
        })?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Transient("model response could not be read".into()))?;
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

fn is_transient(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::StatusCode(code) => *code == 429 || *code >= 500,
        ureq::Error::BadUri(_) | ureq::Error::InvalidProxyUrl | ureq::Error::RedirectFailed => {
            false
        }
        _ => true,
    }
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

    fn transient() -> anyhow::Error {
        anyhow::Error::new(Transient("status 503".into()))
    }

    #[test]
    fn retry_repeats_only_transient_failures_and_stops_after_the_limit() {
        let mut calls = 0;
        let result = retry(3, |_| {
            calls += 1;
            if calls < 3 {
                Err(transient())
            } else {
                Ok(serde_json::json!({"ok":true}))
            }
        });
        assert_eq!(result.unwrap()["ok"], true);
        assert_eq!(calls, 3);

        let mut calls = 0;
        let error = retry(3, |_| {
            calls += 1;
            Err(transient())
        })
        .unwrap_err();
        assert_eq!(calls, 3);
        assert!(format!("{error:#}").contains("after 3 attempts"));

        let mut calls = 0;
        assert!(retry(3, |_| {
            calls += 1;
            Err(anyhow::anyhow!("status 401 (credential rejected)"))
        })
        .is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn only_rate_limits_provider_errors_and_transport_failures_are_transient() {
        for code in [429, 500, 503] {
            assert!(is_transient(&ureq::Error::StatusCode(code)), "{code}");
        }
        for code in [400, 401, 403, 404, 422] {
            assert!(!is_transient(&ureq::Error::StatusCode(code)), "{code}");
        }
        assert!(is_transient(&ureq::Error::ConnectionFailed));
    }

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
