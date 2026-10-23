use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use orders_auth::sessions::{InvalidRefreshToken, MemoryStore, RefreshTokenStore, Sessions, hash};
use orders_auth::settings::Settings;
use orders_auth::tokens::{InvalidToken, issue_access_token, verify_access_token};

const NOW: u64 = 1_792_000_000;

fn settings(extra: &[(&str, &str)]) -> Result<Settings, orders_auth::settings::SettingsError> {
    let mut env: HashMap<&str, &str> = HashMap::from([
        ("APP_AUTH_SIGNING_KEY", "a-test-key-of-thirty-two-bytes!!"),
        ("APP_AUTH_ISSUER", "https://api.example.com/"),
        ("APP_AUTH_AUDIENCE", "orders"),
    ]);
    env.extend(extra.iter().copied());
    Settings::load(|name| env.get(name).map(|value| (*value).to_owned()))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970")
        .as_secs()
}

fn sessions() -> Sessions<MemoryStore> {
    Sessions {
        store: MemoryStore::default(),
        refresh_token_ttl_seconds: 14 * 24 * 3600,
    }
}

#[test]
fn an_access_token_lives_900_seconds_unless_the_environment_says_otherwise() {
    assert_eq!(settings(&[]).unwrap().access_token_ttl_seconds, 900);
    let set = settings(&[("APP_AUTH_ACCESS_TOKEN_TTL_SECONDS", "3600")]).unwrap();
    assert_eq!(set.access_token_ttl_seconds, 3600);
    assert!(settings(&[("APP_AUTH_ACCESS_TOKEN_TTL_SECONDS", "0")]).is_err());
}

#[test]
fn a_valid_access_token_gives_its_user() {
    let settings = settings(&[]).unwrap();
    let token = issue_access_token("ana", &settings, now()).unwrap();

    assert_eq!(verify_access_token(&token, &settings), Ok("ana".to_owned()));
}

#[test]
fn an_expired_access_token_is_invalid() {
    let settings = settings(&[]).unwrap();
    let token = issue_access_token("ana", &settings, now() - 3600).unwrap();

    assert_eq!(verify_access_token(&token, &settings), Err(InvalidToken));
}

#[test]
fn an_access_token_for_another_audience_or_key_is_invalid() {
    let settings = settings(&[]).unwrap();
    let other_audience = settings_with(&settings, |s| s.audience = "billing".into());
    let other_key = settings_with(&settings, |s| s.signing_key = vec![b'x'; 32]);

    for other in [other_audience, other_key] {
        let token = issue_access_token("ana", &other, now()).unwrap();
        assert_eq!(verify_access_token(&token, &settings), Err(InvalidToken));
    }
}

fn settings_with(base: &Settings, change: impl Fn(&mut Settings)) -> Settings {
    let mut changed = base.clone();
    change(&mut changed);
    changed
}

#[test]
fn a_refresh_token_is_replaced_and_the_store_keeps_its_hash() {
    let mut sessions = sessions();
    let first = sessions.open("ana", NOW);

    let (user_id, second) = sessions.rotate(&first, NOW).unwrap();

    assert_eq!(user_id, "ana");
    assert_ne!(second, first);
    assert!(sessions.store.get(&second).is_none());
    assert!(sessions.store.get(&hash(&second)).is_some());
}

#[test]
fn a_refresh_token_used_twice_ends_the_session() {
    let mut sessions = sessions();
    let first = sessions.open("ana", NOW);
    let (_, second) = sessions.rotate(&first, NOW).unwrap();

    assert_eq!(sessions.rotate(&first, NOW), Err(InvalidRefreshToken));
    assert_eq!(sessions.rotate(&second, NOW), Err(InvalidRefreshToken));
}

#[test]
fn a_token_is_marked_used_once() {
    let mut sessions = sessions();
    let token_hash = hash(&sessions.open("ana", NOW));

    let marked = [
        sessions.store.mark_used(&token_hash),
        sessions.store.mark_used(&token_hash),
    ];

    assert_eq!(marked, [true, false]);
}

#[test]
fn an_expired_or_closed_refresh_token_is_invalid() {
    let mut sessions = sessions();
    let expiring = sessions.open("ana", NOW);
    let closing = sessions.open("ana", NOW);

    sessions.close(&closing);

    assert_eq!(
        sessions.rotate(&expiring, NOW + 14 * 24 * 3600),
        Err(InvalidRefreshToken)
    );
    assert_eq!(sessions.rotate(&closing, NOW), Err(InvalidRefreshToken));
}
