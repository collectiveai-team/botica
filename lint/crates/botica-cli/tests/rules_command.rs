use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn project(name: &str, config: Option<&str>) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-command")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("create the project");
    if let Some(text) = config {
        fs::write(dir.join("botica.toml"), text).expect("write the config");
    }
    dir
}

fn botica(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

fn line_of<'a>(text: &'a str, slug: &str) -> &'a str {
    text.lines()
        .find(|line| line.split_whitespace().any(|word| word == slug))
        .unwrap_or_else(|| panic!("no line for {slug} in:\n{text}"))
}

#[test]
fn lists_the_active_rules_with_their_code_and_why() {
    let dir = project("default", None);
    let output = botica(&dir, &["rules"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout(&output);
    let line = line_of(&text, "file-size-guard");
    assert!(
        line.contains("BTSTRUC001") && line.contains("selected by `default`"),
        "{line}"
    );
    assert!(line_of(&text, "io-error").contains("BT001"));
}

#[test]
fn all_adds_the_inactive_rules_and_says_why() {
    let dir = project(
        "ignored",
        Some("ignore = [\"no-utils\"]\n[rules.file-size-guard]\nenabled = false\n"),
    );
    let default_view = stdout(&botica(&dir, &["rules"]));
    assert!(
        !default_view.contains("no-utils") && !default_view.contains("file-size-guard"),
        "{default_view}"
    );

    let text = stdout(&botica(&dir, &["rules", "--all"]));
    assert!(
        line_of(&text, "no-utils").contains("ignored by `no-utils`"),
        "{text}"
    );
    assert!(
        line_of(&text, "file-size-guard").contains("enabled = false"),
        "{text}"
    );
    assert!(line_of(&text, "io-error").contains("selected by"), "{text}");
}

#[test]
fn says_which_config_was_used() {
    let dir = project("with-config", Some("select = [\"BTSTRUC\"]\n"));
    let nested = dir.join("a/b");
    fs::create_dir_all(&nested).expect("create the nested directory");
    let text = stdout(&botica(&nested, &["rules"]));
    assert!(text.contains("botica.toml"), "{text}");
    assert!(
        !text.contains("io-error"),
        "only the structure family is selected:\n{text}"
    );

    let text = stdout(&botica(&project("no-config", None), &["rules"]));
    assert!(text.contains("defaults"), "{text}");
}

#[test]
fn an_explicit_config_wins() {
    let dir = project("explicit", Some("select = [\"all\"]\n"));
    fs::write(dir.join("other.toml"), "select = [\"BT001\"]\n").expect("write the other config");
    let text = stdout(&botica(&dir, &["rules", "--config", "other.toml"]));
    assert!(text.contains("other.toml"));
    assert!(
        text.contains("io-error") && !text.contains("no-utils"),
        "{text}"
    );
}

#[test]
fn an_invalid_config_exits_with_2_and_names_the_problem() {
    let dir = project("invalid", Some("select = [\"no-such-rule\"]\n"));
    let output = botica(&dir, &["rules"]);
    assert_eq!(output.status.code(), Some(2));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(
        message.contains("no-such-rule") && message.contains("select[0]"),
        "{message}"
    );
}

fn json(dir: &Path, args: &[&str]) -> serde_json::Value {
    let output = botica(dir, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("a JSON document")
}

fn rule<'a>(document: &'a serde_json::Value, slug: &str) -> &'a serde_json::Value {
    document["rules"]
        .as_array()
        .expect("a list of rules")
        .iter()
        .find(|rule| rule["rule"] == slug)
        .unwrap_or_else(|| panic!("no {slug} in {document}"))
}

#[test]
fn json_describes_each_active_rule_with_its_resolved_settings() {
    let dir = project("json", None);
    let document = json(&dir, &["rules", "--json"]);
    assert_eq!(document["schema_version"], 1);
    assert_eq!(document["config"]["path"], serde_json::Value::Null);
    assert!(document["config"]["root"].is_string());

    let guard = rule(&document, "file-size-guard");
    assert_eq!(guard["code"], "BTSTRUC001");
    assert_eq!(guard["family"], "structure");
    assert_eq!(guard["summary"], "Keep source files small");
    assert_eq!(guard["status"], "stable");
    assert_eq!(guard["active"], true);
    assert_eq!(guard["reason"], "selected by `default`");
    assert_eq!(guard["severity"], "error");
    assert_eq!(guard["languages"], serde_json::json!(["*"]));
    assert_eq!(
        guard["options"],
        serde_json::json!({"warn": 400, "max": 700})
    );
    assert_eq!(
        rule(&document, "io-error")["family"],
        serde_json::Value::Null
    );
}

#[test]
fn json_does_not_carry_the_ces_of_a_rule() {
    let document = json(&project("json-ces", None), &["rules", "--json"]);
    assert!(
        rule(&document, "file-size-guard").get("ces").is_none(),
        "the CES is documentation of the rule, not part of a public format"
    );
}

#[test]
fn json_follows_the_text_listing_and_all_adds_the_inactive_ones() {
    let dir = project("json-all", Some("ignore = [\"no-utils\"]\n"));
    let active = json(&dir, &["rules", "--json"]);
    assert!(active["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .all(|rule| rule["active"] == true));
    assert!(!active.to_string().contains("no-utils"));

    let all = json(&dir, &["rules", "--json", "--all"]);
    let ignored = rule(&all, "no-utils");
    assert_eq!(ignored["active"], false);
    assert_eq!(ignored["reason"], "ignored by `no-utils`");
}

#[test]
fn json_shows_the_severity_and_options_the_config_gives() {
    let dir = project(
        "json-config",
        Some("[rules.file-size-guard]\nseverity = \"warning\"\nmax = 900\n"),
    );
    let document = json(&dir, &["rules", "--json"]);
    let guard = rule(&document, "file-size-guard");
    assert_eq!(guard["severity"], "warning");
    assert_eq!(guard["options"]["max"], 900);
    assert!(document["config"]["path"]
        .as_str()
        .is_some_and(|path| path.ends_with("botica.toml")));
}

#[test]
fn json_says_where_each_ignore_comes_from() {
    let dir = project(
        "json-ignores",
        Some("[rules.file-size-guard]\nextend-ignores = [\"docs/**\"]\n"),
    );
    let document = json(&dir, &["rules", "--json"]);
    let ignores = rule(&document, "file-size-guard")["ignores"]
        .as_array()
        .expect("ignores")
        .clone();
    assert!(ignores.contains(&serde_json::json!({"glob": "*.lock", "origin": "rule"})));
    assert_eq!(
        ignores.last(),
        Some(&serde_json::json!({"glob": "docs/**", "origin": "config"}))
    );

    let dir = project(
        "json-ignores-replaced",
        Some("[rules.file-size-guard]\nignores = [\"gen/**\"]\n"),
    );
    let document = json(&dir, &["rules", "--json"]);
    assert_eq!(
        rule(&document, "file-size-guard")["ignores"],
        serde_json::json!([{"glob": "gen/**", "origin": "config"}])
    );
}

#[test]
fn list_is_the_default_action_and_takes_the_same_options() {
    let dir = project("list", None);
    assert_eq!(
        json(&dir, &["rules", "list", "--json", "--all"]),
        json(&dir, &["rules", "--json", "--all"])
    );
    let text = stdout(&botica(&dir, &["rules", "list", "--all"]));
    assert!(text.contains("file-size-guard"), "{text}");
}

#[test]
fn the_selection_options_reach_the_listing() {
    let dir = project("select", None);
    let document = json(&dir, &["rules", "--json", "--select", "no-utils"]);
    let slugs: Vec<&str> = document["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .filter_map(|rule| rule["rule"].as_str())
        .collect();
    assert_eq!(slugs, ["no-utils"]);
}

#[test]
fn the_scope_reaches_the_listing_and_is_the_reason_of_what_it_leaves_out() {
    let dir = project("scope", None);
    let text = stdout(&botica(&dir, &["rules", "--all", "--scope", "project"]));
    assert!(
        line_of(&text, "no-utils").contains("outside `--scope project`"),
        "{text}"
    );
}

#[test]
fn json_says_whether_preview_rules_are_on_and_the_command_line_wins() {
    let dir = project("preview", Some("preview = true\n"));
    let preview = |args: &[&str]| json(&dir, args)["config"]["preview"].clone();
    assert_eq!(preview(&["rules", "--json"]), true);
    assert_eq!(preview(&["rules", "--json", "--no-preview"]), false);
    assert_eq!(
        preview(&["rules", "--json", "--no-preview", "--preview"]),
        true
    );

    let dir = project("preview-off", None);
    assert_eq!(json(&dir, &["rules", "--json"])["config"]["preview"], false);
    assert_eq!(
        json(&dir, &["rules", "--json", "--preview"])["config"]["preview"],
        true
    );
}
