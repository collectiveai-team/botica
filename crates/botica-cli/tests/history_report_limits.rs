//! `botica history report`: the limits of the reader and the lines it accepts at their edge (ADR
//! 0019 decision 9.4).

mod common;

use common::history::{dir, finding, history_dir, report, section, write_downloaded, RunSpec};
use common::stdout;
use serde_json::json;
use std::fs;
use std::path::Path;

fn from(dir: &Path) -> String {
    stdout(&report(
        dir,
        &["--from", dir.join("downloads").to_str().expect("UTF-8")],
    ))
}

fn local(dir: &Path, runs: &str, findings: &str) {
    let folder = history_dir(dir);
    fs::create_dir_all(&folder).expect("create the history");
    fs::write(folder.join("runs.jsonl"), runs).expect("write the runs");
    fs::write(folder.join("findings.jsonl"), findings).expect("write the findings");
}

#[test]
fn a_file_over_the_size_limit_is_skipped_and_named() {
    let dir = dir("big-file");
    local(&dir, "", "");
    let file = fs::File::create(history_dir(&dir).join("runs.jsonl")).expect("create");
    file.set_len(51 * 1024 * 1024).expect("a sparse file");

    let text = stdout(&report(&dir, &[]));

    assert!(
        text.contains("is past the size limit of the reader: skipped, so the history is cut"),
        "{text}"
    );
}

#[test]
fn a_folder_name_that_reorders_text_is_not_a_run() {
    let dir = dir("long-name");
    let name = "100\u{202E}".to_owned();
    write_downloaded(
        &dir.join("downloads"),
        &name,
        true,
        &[RunSpec::new(1, "ci", "full").rule("long-named", 0, None)],
        &[],
    );

    let text = from(&dir);

    assert!(!text.contains("long-named"), "{text}");
    assert!(
        text.contains("is not a folder of a CI run: skipped"),
        "{text}"
    );
}

#[test]
fn a_detail_over_64_kb_fails_the_check_of_its_line() {
    let dir = dir("detail");
    let mut run = RunSpec::new(1, "cli", "full")
        .rule("with-detail", 0, None)
        .line();
    run["detail"] = json!({ "text": "x".repeat(70_000) });
    local(&dir, &format!("{run}\n"), "");

    let text = stdout(&report(&dir, &[]));

    assert!(!text.contains("with-detail"), "{text}");
    assert!(text.contains("1 lines failed their checks"), "{text}");
}

#[test]
fn a_run_that_names_too_many_rules_fails_the_check_of_its_line() {
    let dir = dir("many-rules");
    let mut run = RunSpec::new(1, "cli", "full").line();
    let rules: serde_json::Map<String, serde_json::Value> = (0..5_001)
        .map(|n| {
            (
                format!("rule-{n}"),
                json!({"hits": 0, "suppressed": 0, "baselined": 0, "hash": null}),
            )
        })
        .collect();
    run["rules"] = json!(rules);
    local(&dir, &format!("{run}\n"), "");

    let text = stdout(&report(&dir, &[]));

    assert!(text.contains("1 lines failed their checks"), "{text}");
}

#[test]
fn a_joiner_in_a_branch_or_a_slug_is_a_valid_character() {
    let dir = dir("joiner");
    let mut run = RunSpec::new(1, "cli", "full")
        .rule("zwj\u{200D}rule", 0, None)
        .line();
    run["branch"] = json!("fix/\u{200C}name");
    local(&dir, &format!("{run}\n"), "");

    let text = stdout(&report(&dir, &[]));

    assert!(!text.contains("failed their checks"), "{text}");
    assert!(
        section(&text, "Rules with no findings").contains("zwj\u{200D}rule"),
        "{text}"
    );
}

#[test]
fn the_same_run_line_ending_in_crlf_and_in_lf_is_one_line() {
    let dir = dir("crlf");
    let run = RunSpec::new(1, "cli", "full")
        .rule("crlf-rule", 0, None)
        .line();
    local(&dir, &format!("{run}\r\n{run}\n"), "");

    let text = stdout(&report(&dir, &[]));

    assert!(
        section(&text, "Rules with no findings").contains("crlf-rule: no finding in 1 runs"),
        "{text}"
    );
    assert!(!text.contains("both are ignored"), "{text}");
}

#[test]
fn a_run_found_under_two_folders_is_not_trusted_unless_both_copies_say_so() {
    let dir = dir("one-trusted");
    let (one, two) = (dir.join("one"), dir.join("two"));
    for (folder, trusted) in [(&one, true), (&two, false)] {
        write_downloaded(
            folder,
            "100",
            trusted,
            &[RunSpec::new(1, "ci", "full").rule("half-trusted", 0, None)],
            &[],
        );
    }

    let text = stdout(&report(
        &dir,
        &[
            "--from",
            one.to_str().expect("UTF-8"),
            "--from",
            two.to_str().expect("UTF-8"),
        ],
    ));

    assert!(!text.contains("half-trusted"), "{text}");
}

#[test]
fn markdown_puts_every_value_of_the_history_in_code() {
    let dir = dir("markdown-all");
    let unknown = RunSpec::new(3, "cli", "future-mode").line();
    let mut role = finding(2, "review:a[b]", 1);
    role["refutation"] = json!("duplicate");
    local(
        &dir,
        &format!(
            "{}\n{}\n{unknown}\n",
            RunSpec::new(1, "cli", "full")
                .counted("n[1](u)", (1, 9, 0), None)
                .line(),
            RunSpec::new(2, "review:security", "reviewed")
                .rule("review:a[b]", 1, None)
                .line()
        ),
        &format!("{role}\n"),
    );

    let text = stdout(&report(&dir, &["--format", "markdown"]));

    assert!(text.contains("- `n[1](u)`: 9 suppressed"), "{text}");
    assert!(text.contains("- `review:a[b]`: 1 of 1"), "{text}");
    assert!(text.contains("(`mode future-mode, origin cli`)"), "{text}");
}

#[test]
fn a_symbolic_link_in_place_of_dot_botica_is_not_followed() {
    let dir = dir("dot-botica-link");
    let outside = dir.join("outside");
    common::history::write_in(
        &outside.join("history"),
        &[RunSpec::new(1, "cli", "full").rule("through-botica", 0, None)],
        &[],
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, dir.join(".botica")).expect("link");

        let text = stdout(&report(&dir, &[]));

        assert!(!text.contains("through-botica"), "{text}");
    }
}

#[test]
fn a_folder_of_from_that_cannot_be_read_fails_the_command() {
    let dir = dir("missing-from");

    let output = report(
        &dir,
        &["--from", dir.join("nowhere").to_str().expect("UTF-8")],
    );

    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn a_ci_run_with_a_head_that_the_clone_lacks_is_paired_by_time() {
    let dir = dir("no-commit");
    common::history::write_local(
        &dir,
        &[RunSpec::new(1, "hook", "staged")
            .ago(7200)
            .commit("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .rule("no-utils", 0, Some(common::history::HASH))],
        &[finding(1, "no-utils", 1)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .rule("no-utils", 0, Some(common::history::HASH))],
        &[],
    );

    let text = from(&dir);

    assert!(
        section(&text, "Caught before the commit").contains("no-utils: 1"),
        "{text}"
    );
}

#[test]
fn the_pairing_compares_the_newest_500_runs_of_a_branch_and_says_how_many_it_left_out() {
    let dir = dir("many-runs");
    let hooks: Vec<RunSpec> = (1..=502)
        .map(|n| {
            RunSpec::new(n, "hook", "staged")
                .ago(100_000 - n * 10)
                .commit("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                .rule("no-utils", 0, Some(common::history::HASH))
        })
        .collect();
    common::history::write_local(&dir, &hooks, &[]);
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(900, "ci", "diff")
            .ago(10)
            .head("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
            .rule("no-utils", 0, Some(common::history::HASH))],
        &[],
    );

    let text = from(&dir);

    let limits = section(&text, "Pairing limits");
    assert!(
        limits.contains("2 older runs were left out of the pairing"),
        "{text}"
    );
    assert!(
        section(&text, "Caught before the commit").contains("500 of 502 hook runs"),
        "{text}"
    );
}
