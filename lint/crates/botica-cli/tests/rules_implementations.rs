//! `botica rules` and `botica explain` say how each rule is implemented, and `botica rules` names
//! the programs that `botica lint` runs and the tools that enforce rules outside it.

mod common;

use std::path::Path;

use common::extension::{add_rule, folder, implementation, project, rule_toml, README};
use common::{command, stderr, stdout, write};

/// Adds `acme/py-rule`, whose Python script is `check.py`.
fn add_python_rule(dir: &Path) {
    let root = folder("py-rule");
    write(
        dir,
        &format!("{root}/rule.toml"),
        &rule_toml(
            "py-rule",
            "ACMESTRUC002",
            "",
            &implementation("python", "\"python\"", "file = \"check.py\""),
        ),
    );
    write(dir, &format!("{root}/README.md"), README);
    write(dir, &format!("{root}/check.py"), "");
}

fn run(dir: &Path, args: &[&str]) -> String {
    let output = command(dir, args);
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output)
}

fn rule<'a>(document: &'a serde_json::Value, slug: &str) -> &'a serde_json::Value {
    document["rules"]
        .as_array()
        .expect("a list of rules")
        .iter()
        .find(|rule| rule["rule"] == slug)
        .unwrap_or_else(|| panic!("no {slug} in {document}"))
}

const EXTERNAL: &str = "preview = true\nextend-select = [\"cognitive-complexity\"]\n";

#[test]
fn json_lists_each_implementation_with_its_kind_and_what_it_runs() {
    let dir = project("json-implementations", EXTERNAL);
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    add_python_rule(&dir);

    let document: serde_json::Value =
        serde_json::from_str(&run(&dir, &["rules", "--json"])).expect("a JSON document");

    assert_eq!(
        rule(&document, "file-size-guard")["implementations"],
        serde_json::json!([{"kind": "native", "languages": ["*"], "status": "stable",
            "runtime": null, "runs": null, "tool": null}])
    );
    assert_eq!(
        rule(&document, "cognitive-complexity")["implementations"],
        serde_json::json!([{"kind": "external", "languages": ["python"], "status": "preview",
            "runtime": null, "runs": null, "tool": "complexipy"}])
    );
    let script = format!("{}/ext.sh", folder("by-sh"));
    assert_eq!(
        rule(&document, "acme/by-sh")["implementations"][0]["runs"],
        serde_json::json!(["sh", script])
    );
    assert_eq!(
        rule(&document, "acme/by-sh")["implementations"][0]["runtime"],
        "command"
    );
    let check = format!("{}/check.py", folder("py-rule"));
    assert_eq!(
        rule(&document, "acme/py-rule")["implementations"][0]["runs"],
        serde_json::json!(["uv", "run", "--script", check])
    );
}

#[test]
fn the_listing_names_the_programs_lint_runs_and_the_tools_outside_it() {
    let dir = project("programs", EXTERNAL);
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    add_python_rule(&dir);

    let text = run(&dir, &["rules"]);

    let runs = format!(
        "programs that `botica lint` runs:\n  acme/by-sh  sh {by_sh}/ext.sh\n  \
         acme/py-rule  uv run --script {py}/check.py\n",
        by_sh = folder("by-sh"),
        py = folder("py-rule"),
    );
    assert!(text.contains(&runs), "{text}");
    assert!(
        text.contains(
            "tools that enforce rules outside `botica lint`:\n  cognitive-complexity  complexipy\n"
        ),
        "{text}"
    );
}

#[test]
fn the_listing_names_no_program_of_a_rule_that_is_not_active() {
    let dir = project("programs-inactive", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");

    let text = run(&dir, &["rules", "--all", "--ignore", "acme/by-sh"]);

    assert!(!text.contains("programs that"), "{text}");
    assert!(!text.contains("tools that"), "{text}");
}

#[test]
fn explain_says_the_kind_of_each_implementation_and_what_it_runs() {
    let dir = project("explain-implementations", EXTERNAL);
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");

    let kinds = |slug: &str| -> Vec<String> {
        run(&dir, &["explain", slug])
            .lines()
            .filter(|line| line.starts_with("kind:"))
            .map(str::to_owned)
            .collect()
    };

    assert_eq!(kinds("file-size-guard"), ["kind:      native (*)"]);
    assert_eq!(
        kinds("cognitive-complexity"),
        ["kind:      external (python), tool complexipy"]
    );
    assert_eq!(
        kinds("acme/by-sh"),
        [format!(
            "kind:      extension (python), runs sh {}/ext.sh",
            folder("by-sh")
        )]
    );
}
