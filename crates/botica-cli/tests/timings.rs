mod common;

use common::{command, fake_extension, lines, project_in, stderr, stdout, write};
use std::path::PathBuf;
use std::process::Output;

fn project(name: &str) -> PathBuf {
    let dir = project_in("timings", name, None);
    write(&dir, "a.py", "x = 1\n");
    write(&dir, "b.py", "x = 2\n");
    write(&dir, "utils.py", "x = 3\n");
    dir
}

fn lint(dir: &std::path::Path, args: &[&str]) -> Output {
    let mut all = vec!["lint"];
    all.extend_from_slice(args);
    command(dir, &all)
}

/// The words of the `[timing] rule <id> …` line of one rule.
fn rule_row(output: &Output, id: &str) -> Vec<String> {
    stderr(output)
        .lines()
        .filter_map(|line| line.strip_prefix("[timing] rule "))
        .map(|rest| {
            rest.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .find(|words| words.first().is_some_and(|word| word == id))
        .unwrap_or_else(|| panic!("no timing row for {id} in:\n{}", stderr(output)))
}

#[test]
fn without_timings_nothing_is_printed_on_stderr() {
    let dir = project("off");
    assert_eq!(stderr(&lint(&dir, &["."])), "");
}

#[test]
fn each_native_rule_gets_its_time_and_the_files_it_looked_at() {
    let dir = project("native");
    let output = lint(&dir, &[".", "--no-astgrep", "--timings"]);

    // `file-size-guard` reads every file; `no-utils` applies to the python files too.
    let guard = rule_row(&output, "file-size-guard");
    assert_eq!(guard[1], "native", "{guard:?}");
    assert_eq!(guard[3], "ms", "{guard:?}");
    assert!(guard[2].parse::<f64>().is_ok(), "{guard:?}");
    assert_eq!(guard[4..], ["3", "files"], "{guard:?}");
    assert_eq!(rule_row(&output, "no-utils")[4..], ["3", "files"]);
}

#[test]
fn the_slowest_rule_comes_first() {
    let dir = project("order");
    let output = lint(&dir, &[".", "--timings"]);
    let times: Vec<f64> = stderr(&output)
        .lines()
        .filter_map(|line| line.strip_prefix("[timing] rule "))
        .filter_map(|rest| rest.split_whitespace().nth(2)?.parse().ok())
        .collect();
    assert!(times.len() >= 3, "{}", stderr(&output));
    assert!(times.windows(2).all(|pair| pair[0] >= pair[1]), "{times:?}");
}

#[test]
fn ast_grep_rules_are_timed_one_by_one_too() {
    let dir = project("astgrep");
    write(&dir, "log.py", "print('x')\n");
    let output = lint(&dir, &[".", "--timings"]);

    let print = rule_row(&output, "log-no-print");
    assert_eq!(print[1], "syntax", "{print:?}");
    assert!(print[2].parse::<f64>().is_ok(), "{print:?}");
    assert!(
        stdout(&output).contains("[log-no-print]"),
        "the findings are not affected"
    );
}

#[test]
fn the_time_of_the_suppression_comments_is_its_own_row() {
    let dir = project("suppressions");
    let output = lint(&dir, &[".", "--no-astgrep", "--timings"]);
    let row = rule_row(&output, "suppression-comments");
    assert_eq!(row[1], "suppressions", "{row:?}");
}

#[test]
fn an_extension_is_timed_as_a_whole() {
    let dir = project("extension");
    fake_extension(&dir, "");
    let output = lint(
        &dir,
        &[".", "--no-astgrep", "--ext", "sh ext.sh", "--timings"],
    );

    let text = stderr(&output);
    let row = text
        .lines()
        .find_map(|line| line.strip_prefix("[timing] extension `sh ext.sh`"))
        .unwrap_or_else(|| panic!("no extension row in:\n{text}"));
    for part in ["start", "lint", "shutdown", "total"] {
        assert!(row.contains(part), "{part} missing in `{row}`");
    }
}

#[test]
fn timings_never_touch_what_goes_to_stdout() {
    let dir = project("stdout");
    write(&dir, "big.py", &lines(701));
    let plain = lint(&dir, &[".", "--output-format", "json"]);
    let timed = lint(&dir, &[".", "--output-format", "json", "--timings"]);
    assert_eq!(stdout(&plain), stdout(&timed));
    serde_json::from_str::<serde_json::Value>(&stdout(&timed)).expect("one JSON document");
    assert_eq!(plain.status.code(), timed.status.code());
}
