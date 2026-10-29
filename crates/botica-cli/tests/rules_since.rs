use std::process::{Command, Output};

fn botica(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(args)
        .current_dir(env!("CARGO_TARGET_TMPDIR"))
        .output()
        .expect("run botica")
}

fn stdout(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

#[test]
fn lists_one_line_per_change_of_each_release_after_the_version() {
    let text = stdout(&botica(&["rules", "--since", "0.12.0"]));
    let lines: Vec<&str> = text.lines().collect();

    for expected in [
        "0.13.0  database-package (BTDB001)  renamed from arch-database-package",
        "0.13.0  database-package (BTDB001)  code BTARCH001 -> BTDB001",
        "0.13.0  db-no-create-all (BTDB002)  code BTARCH005 -> BTDB002",
        "0.18.0  botica-version-aligned (BT007)  added as preview",
    ] {
        assert!(lines.contains(&expected), "no `{expected}` in:\n{text}");
    }
    assert!(
        lines.iter().all(|line| !line.starts_with("0.12.0")),
        "{text}"
    );
}

#[test]
fn a_removed_rule_names_the_rule_that_replaces_it() {
    let text = stdout(&botica(&["rules", "--since", "0.9.0"]));

    assert!(
        text.lines()
            .any(|line| line
                == "0.10.0  no-console-log (BTLOG003)  removed, replaced by log-no-print"),
        "{text}"
    );
}

#[test]
fn json_prints_one_document_with_the_changes() {
    let text = stdout(&botica(&["rules", "--since", "0.17.0", "--json"]));
    let document: serde_json::Value = serde_json::from_str(&text).expect("one JSON document");

    assert_eq!(document["since"], "0.17.0");
    assert_eq!(document["until"], env!("CARGO_PKG_VERSION"));
    let added = document["changes"]
        .as_array()
        .expect("a list of changes")
        .iter()
        .find(|change| change["rule"] == "botica-version-aligned")
        .expect("0.18.0 added botica-version-aligned");
    assert_eq!(added["change"], "added");
    assert_eq!(added["status"], "preview");
    assert_eq!(added["code"], "BT007");
}

#[test]
fn a_version_newer_than_the_binary_lists_nothing() {
    let text = stdout(&botica(&["rules", "--since", "999.0.0"]));

    assert_eq!(text, "");
}

#[test]
fn a_version_that_is_not_three_numbers_exits_2() {
    let output = botica(&["rules", "--since", "0.13"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a version like `0.13.0`"), "{stderr}");
}
