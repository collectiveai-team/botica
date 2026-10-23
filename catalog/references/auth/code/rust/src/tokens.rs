//! The access tokens that the service signs and verifies, with `jsonwebtoken`.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::settings::Settings;

const ALGORITHM: Algorithm = Algorithm::HS256;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    iss: String,
    aud: String,
    iat: u64,
    exp: u64,
}

/// The token is malformed, signed with another key, expired or meant for someone else.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidToken;

/// Sign an access token for the user that expires after the lifetime of the settings. `now` is
/// a number of seconds since the Unix epoch.
pub fn issue_access_token(
    user_id: &str,
    settings: &Settings,
    now: u64,
) -> jsonwebtoken::errors::Result<String> {
    let claims = Claims {
        sub: user_id.to_owned(),
        iss: settings.issuer.clone(),
        aud: settings.audience.clone(),
        iat: now,
        exp: now + settings.access_token_ttl_seconds,
    };
    encode(
        &Header::new(ALGORITHM),
        &claims,
        &EncodingKey::from_secret(&settings.signing_key),
    )
}

/// Return the user of an access token of the service, after checking its algorithm, signature,
/// issuer, audience and expiry.
pub fn verify_access_token(token: &str, settings: &Settings) -> Result<String, InvalidToken> {
    let mut validation = Validation::new(ALGORITHM);
    validation.set_issuer(&[&settings.issuer]);
    validation.set_audience(&[&settings.audience]);
    validation.set_required_spec_claims(&["exp", "iat", "sub", "iss", "aud"]);
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(&settings.signing_key),
        &validation,
    )
    .map(|data| data.claims.sub)
    .map_err(|_| InvalidToken)
}
