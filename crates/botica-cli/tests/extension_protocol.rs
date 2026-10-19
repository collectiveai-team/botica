//! What `botica lint` says to the process of an extension rule: the rules it asks for and their
//! options, `lint/project` for a rule of project scope, and the check of the rules that the
//! process declares when it starts.

mod common;

use std::path::Path;

use common::extension::{folder, implementation, lint, project, rule_toml, README};
use common::{stderr, stdout, write};

/// An extension that declares `declares`, writes each request it gets to `requests.log`, and
/// answers `lint/files` with no finding and `lint/project` with one finding in `app.py`.
fn logging_script(declares: &str) -> String {
    format!(
        r#"while IFS= read -r line; do
  printf '%s\n' "$line" >> requests.log
  case "$line" in
    *'"initialize"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":1,"result":{{"rules":[{declares}]}}}}' ;;
    *'"lint/files"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":2,"result":{{"diagnostics":[]}}}}' ;;
    *'"lint/project"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":2,"result":{{"diagnostics":[{{"path":"app.py","rule":"acme/by-sh","severity":"error","start":4,"end":5,"message":"project"}}]}}}}' ;;
    *'"shutdown"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":3,"result":null}}'; exit 0 ;;
  esac
done
"#
    )
}

const DECLARES_BY_SH: &str = r#"{"id":"acme/by-sh","languages":["python"]}"#;

/// Adds `acme/by-sh`, of `scope`, with the option `limit` (3 by default), run by `script`.
fn add_logging_rule(dir: &Path, scope: &str, script: &str) {
    let root = folder("by-sh");
    let how = format!("command = [\"sh\", \"{root}/ext.sh\"]");
    let text = rule_toml(
        "by-sh",
        "ACMESTRUC001",
        "\n[options]\nlimit = { type = \"int\", default = 3 }",
        &implementation("command", "\"python\"", &how),
    )
    .replace("scope = \"file\"", &format!("scope = \"{scope}\""));
    write(dir, &format!("{root}/rule.toml"), &text);
    write(dir, &format!("{root}/README.md"), README);
    write(dir, &format!("{root}/ext.sh"), script);
}

fn request(dir: &Path, method: &str) -> serde_json::Value {
    let log = std::fs::read_to_string(dir.join("requests.log")).unwrap_or_default();
    let line = log
        .lines()
        .find(|line| line.contains(&format!("\"method\":\"{method}\"")))
        .unwrap_or_else(|| panic!("no `{method}` in:\n{log}"));
    serde_json::from_str(line).expect("a JSON request")
}

#[test]
fn lint_files_names_the_rule_and_carries_the_options_the_configuration_resolves() {
    let dir = project("protocol-options", "[rules.\"acme/by-sh\"]\nlimit = 5\n");
    add_logging_rule(&dir, "file", &logging_script(DECLARES_BY_SH));

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let params = &request(&dir, "lint/files")["params"];
    assert_eq!(params["rules"], serde_json::json!(["acme/by-sh"]));
    assert_eq!(
        params["options"],
        serde_json::json!({"acme/by-sh": {"limit": 5}})
    );
    assert_eq!(params["files"][0]["path"], "app.py");
}

#[test]
fn a_rule_of_project_scope_gets_lint_project_with_the_root_and_the_paths() {
    let dir = project("protocol-project", "");
    add_logging_rule(&dir, "project", &logging_script(DECLARES_BY_SH));

    let output = lint(&dir, &[]);

    assert_eq!(
        stdout(&output),
        "app.py:1:5: error [acme/by-sh] project\n",
        "{}",
        stderr(&output)
    );
    let params = &request(&dir, "lint/project")["params"];
    assert_eq!(params["root"], dir.display().to_string());
    assert_eq!(params["files"], serde_json::json!(["app.py"]));
    assert_eq!(params["rules"], serde_json::json!(["acme/by-sh"]));
    assert_eq!(
        params["options"],
        serde_json::json!({"acme/by-sh": {"limit": 3}})
    );
    let log = std::fs::read_to_string(dir.join("requests.log")).expect("the log");
    assert!(!log.contains("\"lint/files\""), "{log}");
}

#[test]
fn an_extension_that_does_not_declare_its_rule_fails_and_says_so() {
    let dir = project("protocol-undeclared", "");
    add_logging_rule(&dir, "file", &logging_script(""));

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(
        message.contains("does not declare `acme/by-sh`"),
        "{message}"
    );
    let log = std::fs::read_to_string(dir.join("requests.log")).expect("the log");
    assert!(!log.contains("\"lint/files\""), "{log}");
}

#[test]
fn a_command_that_two_rules_run_may_declare_both() {
    let dir = project("protocol-shared", "");
    let both = format!(r#"{DECLARES_BY_SH},{{"id":"acme/other","languages":["python"]}}"#);
    add_logging_rule(&dir, "file", &logging_script(&both));
    let how = format!("command = [\"sh\", \"{}/ext.sh\"]", folder("by-sh"));
    write(
        &dir,
        &format!("{}/rule.toml", folder("other")),
        &rule_toml(
            "other",
            "ACMESTRUC002",
            "",
            &implementation("command", "\"python\"", &how),
        ),
    );
    write(&dir, &format!("{}/README.md", folder("other")), README);

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

#[test]
fn an_extension_that_declares_a_rule_its_catalog_does_not_point_to_it_fails() {
    let dir = project("protocol-extra", "");
    let both = format!(r#"{DECLARES_BY_SH},{{"id":"acme/other","languages":["python"]}}"#);
    add_logging_rule(&dir, "file", &logging_script(&both));

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
    let message = stderr(&output);
    assert!(message.contains("declares `acme/other`"), "{message}");
}
