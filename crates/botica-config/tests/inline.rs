use std::fs;
use std::path::{Path, PathBuf};

use botica_config::{discover, inline_setting, load_file};

fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("inline")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("create the project");
    dir
}

fn write(dir: &Path, name: &str, text: &str) {
    fs::write(dir.join(name), text).expect("write the file");
}

fn setting(text: &str) -> toml::Table {
    inline_setting(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

#[test]
fn a_key_value_pair_is_a_setting_and_dotted_keys_reach_inside_tables() {
    assert_eq!(
        setting("select = [\"no-utils\"]")["select"],
        toml::Value::Array(vec![toml::Value::String("no-utils".into())])
    );
    assert_eq!(
        setting("rules.file-size-guard.max = 900")["rules"]["file-size-guard"]["max"],
        toml::Value::Integer(900)
    );
    assert_eq!(
        setting("preview = true")["preview"],
        toml::Value::Boolean(true)
    );
}

#[test]
fn what_is_not_toml_or_not_a_setting_of_botica_is_refused_and_says_why() {
    for (text, expected) in [
        ("select = [no-utils]", "select"),
        ("no-such-key = 1", "no-such-key"),
        ("preview = \"yes\"", "preview"),
        ("botica.toml", "botica.toml"),
        ("", "empty"),
    ] {
        let message = inline_setting(text).expect_err(text).to_string();
        assert!(message.contains(expected), "{text}: {message}");
    }
}

#[test]
fn a_setting_replaces_the_key_of_the_file_and_leaves_the_others() {
    let dir = fresh("replaces");
    write(
        &dir,
        "botica.toml",
        "select = [\"no-utils\"]\nignore = [\"BT001\"]\n",
    );
    let loaded = discover(&dir)
        .expect("config")
        .with_inline(&[setting("select = [\"file-size-guard\"]")])
        .expect("valid");
    assert_eq!(loaded.config.select, ["file-size-guard"]);
    assert_eq!(loaded.config.ignore, ["BT001"]);
    assert!(loaded
        .source
        .is_some_and(|path| path.ends_with("botica.toml")));
}

#[test]
fn tables_are_merged_key_by_key_and_arrays_are_replaced() {
    let dir = fresh("merges");
    write(
        &dir,
        "botica.toml",
        "extend-select = [\"BT001\"]\n[rules.file-size-guard]\nseverity = \"warning\"\nwarn = 10\n",
    );
    let loaded = discover(&dir)
        .expect("config")
        .with_inline(&[
            setting("rules.file-size-guard.max = 900"),
            setting("extend-select = [\"BT003\"]"),
        ])
        .expect("valid");
    let rule = &loaded.config.rules["file-size-guard"];
    assert_eq!(rule.severity, Some(botica_config::RuleSeverity::Warning));
    assert_eq!(rule.options["warn"], toml::Value::Integer(10));
    assert_eq!(rule.options["max"], toml::Value::Integer(900));
    assert_eq!(loaded.config.extend_select, ["BT003"]);
}

#[test]
fn when_a_key_comes_twice_the_last_one_wins() {
    let loaded = discover(&fresh("twice"))
        .expect("config")
        .with_inline(&[
            setting("select = [\"no-utils\"]"),
            setting("select = [\"file-size-guard\"]"),
        ])
        .expect("valid");
    assert_eq!(loaded.config.select, ["file-size-guard"]);
}

#[test]
fn settings_work_without_a_file_and_over_a_pyproject_table() {
    let none = discover(&fresh("none"))
        .expect("defaults")
        .with_inline(&[setting("preview = true")])
        .expect("valid");
    assert!(none.config.preview && none.source.is_none());

    let dir = fresh("pyproject");
    write(
        &dir,
        "pyproject.toml",
        "[project]\nname = \"x\"\n[tool.botica]\nselect = [\"no-utils\"]\npreview = true\n",
    );
    let loaded = discover(&dir)
        .expect("config")
        .with_inline(&[setting("select = [\"BT001\"]")])
        .expect("valid");
    assert_eq!(loaded.config.select, ["BT001"]);
    assert!(loaded.config.preview);
}

#[test]
fn a_file_given_by_path_is_the_base_too() {
    let dir = fresh("by-path");
    write(
        &dir,
        "other.toml",
        "select = [\"no-utils\"]\npreview = true\n",
    );
    let loaded = load_file(&dir.join("other.toml"))
        .expect("config")
        .with_inline(&[setting("preview = false")])
        .expect("valid");
    assert_eq!(loaded.config.select, ["no-utils"]);
    assert!(!loaded.config.preview);
}
