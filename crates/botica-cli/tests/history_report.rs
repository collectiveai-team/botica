//! `botica history report`: the rules with no findings, the rules with many suppressions, the
//! window, the formats and the trust of a source (ADR 0019 decisions 8 and 9).

mod common;

use common::history::{dir, finding, report, section, write_downloaded, write_local, RunSpec};
use common::{stderr, stdout};

#[test]
fn a_rule_that_ran_and_found_nothing_is_listed_and_a_rule_that_did_not_run_is_not() {
    let dir = dir("quiet");
    let runs = [
        RunSpec::new(1, "cli", "full")
            .rule("no-utils", 0, None)
            .rule("file-size-guard", 1, None),
        RunSpec::new(2, "hook", "paths").rule("no-utils", 0, None),
    ];
    write_local(&dir, &runs, &[finding(1, "file-size-guard", 7)]);

    let output = report(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    let quiet = section(&text, "Rules with no findings");
    assert!(quiet.contains("no-utils: no finding in 2 runs"), "{text}");
    assert!(!quiet.contains("file-size-guard"), "{text}");
    assert!(!quiet.contains("conventional-commits"), "{text}");
}

#[test]
fn a_rule_whose_findings_are_all_suppressed_or_baselined_is_not_a_rule_with_no_findings() {
    let dir = dir("silenced");
    let runs = [RunSpec::new(1, "cli", "full")
        .counted("suppressed-only", (0, 2, 0), None)
        .counted("baselined-only", (0, 0, 3), None)
        .counted("nothing", (0, 0, 0), None)];
    write_local(&dir, &runs, &[]);

    let text = stdout(&report(&dir, &[]));

    let quiet = section(&text, "Rules with no findings");
    assert!(quiet.contains("nothing: no finding in 1 runs"), "{text}");
    assert!(!quiet.contains("suppressed-only"), "{text}");
    assert!(!quiet.contains("baselined-only"), "{text}");
}

#[test]
fn the_report_warns_that_zero_findings_can_mean_that_the_rule_is_followed() {
    let dir = dir("warning");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("no-utils", 0, None)],
        &[],
    );

    let text = stdout(&report(&dir, &[]));

    let quiet = section(&text, "Rules with no findings");
    assert!(
        quiet.contains("Zero can mean that the rule works"),
        "{text}"
    );
    assert!(quiet.contains("`AGENTS.md`"), "{text}");
    assert!(quiet.contains("a person decides to retire it"), "{text}");
}

#[test]
fn a_run_outside_the_window_is_left_out() {
    let dir = dir("window");
    let runs = [
        RunSpec::new(1, "cli", "full").rule("rule-recent", 0, None),
        RunSpec::new(2, "cli", "full")
            .ago(10 * 86_400)
            .rule("rule-stale", 0, None),
    ];
    write_local(&dir, &runs, &[]);

    let fortnight = stdout(&report(&dir, &["--days", "14"]));
    let week = stdout(&report(&dir, &["--days", "7"]));
    let day = stdout(&report(&dir, &["--days", "1"]));

    assert!(
        fortnight.contains("rule-recent: no finding in 1 runs"),
        "{fortnight}"
    );
    assert!(
        fortnight.contains("rule-stale: no finding in 1 runs"),
        "{fortnight}"
    );
    assert!(
        week.contains("rule-recent:") && !week.contains("rule-stale:"),
        "{week}"
    );
    assert!(day.contains("rule-recent: no finding in 1 runs"), "{day}");
    assert!(!day.contains("rule-stale:"), "{day}");
}

#[test]
fn a_rule_with_many_suppressions_is_listed_by_its_share_of_suppressed_findings() {
    let dir = dir("noisy");
    let runs = [
        RunSpec::new(1, "cli", "full")
            .counted("often-silenced", (2, 6, 0), None)
            .counted("rarely-silenced", (50, 6, 0), None)
            .counted("few", (0, 2, 0), None),
        RunSpec::new(2, "cli", "full").counted("often-silenced", (0, 2, 0), None),
    ];
    write_local(&dir, &runs, &[]);

    let text = stdout(&report(&dir, &[]));

    let noisy = section(&text, "Rules with many suppressions");
    assert!(
        noisy.contains("often-silenced: 8 suppressed, 2 reported (80%)"),
        "{text}"
    );
    assert!(!noisy.contains("rarely-silenced"), "{text}");
    assert!(!noisy.contains("few"), "{text}");
}

#[test]
fn the_thresholds_of_the_noise_are_options() {
    let dir = dir("thresholds");
    let runs = [RunSpec::new(1, "cli", "full").counted("few", (1, 2, 0), None)];
    write_local(&dir, &runs, &[]);

    let default = stdout(&report(&dir, &[]));
    let lower = stdout(&report(
        &dir,
        &["--min-suppressed", "2", "--min-share", "60"],
    ));

    assert!(!section(&default, "Rules with many suppressions").contains("few:"));
    assert!(
        section(&lower, "Rules with many suppressions")
            .contains("few: 2 suppressed, 1 reported (67%)"),
        "{lower}"
    );
}

#[test]
fn a_run_of_a_source_that_is_not_trusted_feeds_neither_the_rules_with_no_findings_nor_the_noise() {
    let dir = dir("untrusted");
    let downloads = dir.join("downloads");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("local-rule", 0, None)],
        &[],
    );
    write_downloaded(
        &downloads,
        "100",
        false,
        &[RunSpec::new(2, "ci", "full")
            .counted("forged-quiet", (0, 0, 0), None)
            .counted("forged-noisy", (0, 40, 0), None)
            .counted("local-rule", (9, 0, 0), None)],
        &[],
    );
    write_downloaded(
        &downloads,
        "101",
        true,
        &[RunSpec::new(3, "ci", "full").rule("ci-rule", 0, None)],
        &[],
    );

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    let quiet = section(&text, "Rules with no findings");
    assert!(quiet.contains("local-rule: no finding in 1 runs"), "{text}");
    assert!(quiet.contains("ci-rule"), "{text}");
    assert!(!quiet.contains("forged-quiet"), "{text}");
    assert!(
        !section(&text, "Rules with many suppressions").contains("forged-noisy"),
        "{text}"
    );
    let read = section(&text, "Runs read");
    assert!(read.contains("1 runs of the local history"), "{text}");
    assert!(read.contains("1 runs of CI from trusted sources"), "{text}");
    assert!(
        read.contains("1 runs of CI from sources that are not trusted"),
        "{text}"
    );
}

#[test]
fn a_downloaded_run_with_no_source_file_is_not_trusted() {
    let dir = dir("no-source");
    let downloads = dir.join("downloads");
    write_downloaded(
        &downloads,
        "100",
        true,
        &[RunSpec::new(2, "ci", "full").rule("forged", 0, None)],
        &[],
    );
    std::fs::remove_file(downloads.join("100").join("source.json")).expect("remove source.json");

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        !section(&text, "Rules with no findings").contains("forged"),
        "{text}"
    );
    assert!(
        text.contains("CI run 100 has no source.json: not trusted"),
        "{text}"
    );
}

#[test]
fn two_runs_of_different_sources_with_one_run_id_stay_two_runs() {
    let dir = dir("sources");
    let downloads = dir.join("downloads");
    let first = RunSpec::new(7, "ci", "full").rule("first", 0, None);
    let second = RunSpec::new(7, "ci", "full").rule("second", 0, None);
    write_downloaded(&downloads, "100", true, &[first], &[]);
    write_downloaded(&downloads, "101", true, &[second], &[]);

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    let quiet = section(&text, "Rules with no findings");
    assert!(
        quiet.contains("first") && quiet.contains("second"),
        "{text}"
    );
}

#[test]
fn the_markdown_format_has_headings_and_puts_each_value_of_the_history_in_code() {
    let dir = dir("markdown");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("no-utils", 0, None)],
        &[],
    );

    let text = stdout(&report(&dir, &["--format", "markdown"]));

    assert!(
        text.starts_with("# botica history report: the last 28 days"),
        "{text}"
    );
    assert!(text.contains("\n## Rules with no findings\n"), "{text}");
    assert!(
        text.contains("- `no-utils`: no finding in 1 runs"),
        "{text}"
    );
}

#[test]
fn the_text_format_has_no_markup() {
    let dir = dir("text");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("no-utils", 0, None)],
        &[],
    );

    let text = stdout(&report(&dir, &[]));

    assert!(
        text.starts_with("botica history report: the last 28 days"),
        "{text}"
    );
    assert!(
        text.contains("\n  no-utils: no finding in 1 runs"),
        "{text}"
    );
    assert!(!text.contains("## "), "{text}");
}

#[test]
fn a_run_with_no_history_says_so_and_does_not_fail() {
    let dir = dir("empty");

    let output = report(&dir, &[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("0 runs of the local history"), "{text}");
    assert!(
        text.contains("No run of a trusted source in the window."),
        "{text}"
    );
}

#[test]
fn the_history_of_the_root_is_read_from_a_subdirectory() {
    let dir = dir("subdirectory");
    write_local(
        &dir,
        &[RunSpec::new(1, "cli", "full").rule("no-utils", 0, None)],
        &[],
    );
    std::fs::create_dir_all(dir.join("src")).expect("create src");

    let text = stdout(&report(&dir.join("src"), &[]));

    assert!(text.contains("no-utils: no finding in 1 runs"), "{text}");
}

#[test]
fn a_window_of_zero_days_fails() {
    let dir = dir("zero-days");

    let output = report(&dir, &["--days", "0"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("--days must be at least 1"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn the_help_says_what_the_report_reads() {
    let dir = dir("help");

    let text = stdout(&common::history::help(&dir));

    assert!(text.contains("--days"), "{text}");
    assert!(text.contains("--from"), "{text}");
    assert!(text.contains("--format"), "{text}");
}

#[test]
fn the_lines_that_botica_lint_record_writes_are_read_without_a_skipped_line() {
    let dir = common::record::project("round-trip", None);
    common::record::lint(&dir, &["--record"]);
    common::record::lint(&dir, &["--record", "a.py"]);

    let text = stdout(&report(&dir, &[]));

    let read = section(&text, "Runs read");
    assert!(read.contains("2 runs of the local history"), "{text}");
    assert!(!read.contains("failed their checks"), "{text}");
    let quiet = section(&text, "Rules with no findings");
    assert!(quiet.contains("no-utils: no finding in 2 runs"), "{text}");
    assert!(!quiet.contains("unused-suppression"), "{text}");
}
