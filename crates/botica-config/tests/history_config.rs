//! The `[history]` table of the configuration (ADR 0019).

use botica_config::Config;

#[test]
fn without_the_table_nothing_is_recorded_and_no_key_is_required() {
    let config = Config::from_botica_toml("").expect("parses");

    assert!(!config.history.record);
    assert!(!config.history.require_key);
}

#[test]
fn the_table_turns_recording_on_and_requires_the_key() {
    let config =
        Config::from_botica_toml("[history]\nrecord = true\nrequire-key = true\n").expect("parses");

    assert!(config.history.record);
    assert!(config.history.require_key);
}

#[test]
fn the_table_takes_no_other_key() {
    let message = Config::from_botica_toml("[history]\nkeep = 90\n")
        .expect_err("rejected")
        .to_string();

    assert!(message.contains("keep"), "{message}");
}
