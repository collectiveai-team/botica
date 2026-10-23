//! The settings of authentication: the one module that reads the environment for it.

use std::fmt;

/// The key, the issuer, the audience and the lifetimes of the tokens, in seconds.
#[derive(Clone, Debug)]
pub struct Settings {
    pub signing_key: Vec<u8>,
    pub issuer: String,
    pub audience: String,
    pub access_token_ttl_seconds: u64,
    pub refresh_token_ttl_seconds: u64,
}

/// A variable of the environment that is missing or holds a value the settings reject.
#[derive(Debug, PartialEq, Eq)]
pub struct SettingsError(pub String);

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SettingsError {}

impl Settings {
    /// Read the settings through `getenv`, with the prefix `APP_AUTH_`. An access token lives 900
    /// seconds and a refresh token 14 days when the environment names no lifetime.
    pub fn load(getenv: impl Fn(&str) -> Option<String>) -> Result<Self, SettingsError> {
        let signing_key = getenv("APP_AUTH_SIGNING_KEY").unwrap_or_default();
        if signing_key.len() < 32 {
            return Err(SettingsError(
                "APP_AUTH_SIGNING_KEY is shorter than 32 bytes".into(),
            ));
        }
        Ok(Self {
            signing_key: signing_key.into_bytes(),
            issuer: required(&getenv, "APP_AUTH_ISSUER")?,
            audience: required(&getenv, "APP_AUTH_AUDIENCE")?,
            access_token_ttl_seconds: seconds(&getenv, "APP_AUTH_ACCESS_TOKEN_TTL_SECONDS", 900)?,
            refresh_token_ttl_seconds: seconds(
                &getenv,
                "APP_AUTH_REFRESH_TOKEN_TTL_SECONDS",
                14 * 24 * 3600,
            )?,
        })
    }

    /// Read the settings from the environment of the process.
    pub fn from_env() -> Result<Self, SettingsError> {
        Self::load(|name| std::env::var(name).ok())
    }
}

fn required(getenv: &impl Fn(&str) -> Option<String>, name: &str) -> Result<String, SettingsError> {
    getenv(name)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SettingsError(format!("{name} is not set")))
}

fn seconds(
    getenv: &impl Fn(&str) -> Option<String>,
    name: &str,
    fallback: u64,
) -> Result<u64, SettingsError> {
    match getenv(name) {
        None => Ok(fallback),
        Some(raw) => match raw.parse::<u64>() {
            Ok(value) if value > 0 => Ok(value),
            _ => Err(SettingsError(format!(
                "{name} is not a positive number of seconds: {raw:?}"
            ))),
        },
    }
}
