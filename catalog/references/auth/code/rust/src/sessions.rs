//! The refresh tokens of a session: stored as a hash, replaced at each use, revoked when reused.

use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

/// What the server keeps of a refresh token: never the token itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRefreshToken {
    pub user_id: String,
    pub family: String,
    pub expires_at: u64,
    pub used: bool,
}

/// The port of the store of refresh tokens, keyed by the hash of each token.
pub trait RefreshTokenStore {
    /// Store a refresh token, or replace what is stored for its hash.
    fn save(&mut self, token_hash: &str, stored: StoredRefreshToken);
    /// Return what is stored for the hash.
    fn get(&self, token_hash: &str) -> Option<StoredRefreshToken>;
    /// Delete every refresh token of a session.
    fn delete_family(&mut self, family: &str);
}

/// The port in a map, for one process; several processes keep the tokens in a database.
#[derive(Default)]
pub struct MemoryStore {
    by_hash: HashMap<String, StoredRefreshToken>,
}

impl RefreshTokenStore for MemoryStore {
    fn save(&mut self, token_hash: &str, stored: StoredRefreshToken) {
        self.by_hash.insert(token_hash.to_owned(), stored);
    }

    fn get(&self, token_hash: &str) -> Option<StoredRefreshToken> {
        self.by_hash.get(token_hash).cloned()
    }

    fn delete_family(&mut self, family: &str) {
        self.by_hash.retain(|_, stored| stored.family != family);
    }
}

/// The refresh token is unknown, expired or used already; the caller signs in again.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidRefreshToken;

/// Opens sessions, rotates their refresh tokens and ends them.
pub struct Sessions<S: RefreshTokenStore> {
    pub store: S,
    pub refresh_token_ttl_seconds: u64,
}

impl<S: RefreshTokenStore> Sessions<S> {
    /// Start a session for the user and return its first refresh token.
    pub fn open(&mut self, user_id: &str, now: u64) -> String {
        let family = random_token(16);
        self.issue(user_id, &family, now)
    }

    /// Return the user of the token and the refresh token that replaces it. A token that was used
    /// already ends its session: the caller or a thief holds a copy.
    pub fn rotate(
        &mut self,
        token: &str,
        now: u64,
    ) -> Result<(String, String), InvalidRefreshToken> {
        let token_hash = hash(token);
        let stored = match self.store.get(&token_hash) {
            Some(stored) if stored.expires_at > now => stored,
            _ => return Err(InvalidRefreshToken),
        };
        if stored.used {
            self.store.delete_family(&stored.family);
            return Err(InvalidRefreshToken);
        }
        self.store.save(
            &token_hash,
            StoredRefreshToken {
                used: true,
                ..stored.clone()
            },
        );
        let replacement = self.issue(&stored.user_id, &stored.family, now);
        Ok((stored.user_id, replacement))
    }

    /// End the session of the refresh token, if it has one.
    pub fn close(&mut self, token: &str) {
        if let Some(stored) = self.store.get(&hash(token)) {
            self.store.delete_family(&stored.family);
        }
    }

    fn issue(&mut self, user_id: &str, family: &str, now: u64) -> String {
        let token = random_token(32);
        let stored = StoredRefreshToken {
            user_id: user_id.to_owned(),
            family: family.to_owned(),
            expires_at: now + self.refresh_token_ttl_seconds,
            used: false,
        };
        self.store.save(&hash(&token), stored);
        token
    }
}

/// The SHA-256 of a refresh token, in hexadecimal: the key of the store.
pub fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn random_token(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    rand::fill(buffer.as_mut_slice());
    URL_SAFE_NO_PAD.encode(buffer)
}
