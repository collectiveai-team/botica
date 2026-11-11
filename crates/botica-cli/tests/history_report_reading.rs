//! `botica history report`: where the reader looks for the files, what it counts once and what
//! it refuses to print (ADR 0019 decisions 2, 8 and 9).

mod common;

use common::history::{
    dir, finding, history_dir, report, section, write_downloaded, write_in, write_local, RunSpec,
    HASH,
};
use common::stdout;
use serde_json::json;
use std::fs;

const COMMIT_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COMMIT_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn the_files_of_an_artifact_folder_under_the_run_are_read() {
    let dir = dir("nested");
    let downloads = dir.join("downloads");
    let run = downloads.join("100");
    write_in(
        &run.join("botica-history-lint"),
        &[RunSpec::new(1, "ci", "full").rule("nested-rule", 0, None)],
        &[],
    );
    fs::write(
        run.join("source.json"),
        json!({"trusted": true}).to_string(),
    )
    .expect("source");

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        section(&text, "Rules with no findings").contains("nested-rule"),
        "{text}"
    );
}

#[test]
fn a_run_found_under_two_folders_of_from_counts_once() {
    let dir = dir("two-folders");
    let (one, two) = (dir.join("one"), dir.join("two"));
    for folder in [&one, &two] {
        write_downloaded(
            folder,
            "100",
            true,
            &[RunSpec::new(1, "ci", "full").counted("often", (1, 6, 0), None)],
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

    assert!(
        section(&text, "Rules with many suppressions").contains("often: 6 suppressed, 1 reported"),
        "{text}"
    );
}

#[test]
fn two_run_lines_that_differ_only_in_a_key_the_reader_does_not_use_are_both_ignored() {
    let dir = dir("raw");
    let one = RunSpec::new(1, "cli", "full").rule("twin", 0, None).line();
    let mut two = one.clone();
    two["botica"] = json!("9.9.9");
    let folder = history_dir(&dir);
    fs::create_dir_all(&folder).expect("create the history");
    fs::write(folder.join("runs.jsonl"), format!("{one}\n{two}\n")).expect("write");

    let text = stdout(&report(&dir, &[]));

    assert!(
        !section(&text, "Rules with no findings").contains("twin"),
        "{text}"
    );
    assert!(text.contains("both are ignored"), "{text}");
}

#[test]
fn a_hook_run_and_a_ci_run_with_different_keys_are_not_paired() {
    let dir = dir("keys");
    let mut hook = RunSpec::new(1, "hook", "staged")
        .ago(7200)
        .commit(COMMIT_A)
        .rule("no-utils", 0, Some(HASH))
        .line();
    hook["key_id"] = json!("01234567");
    let folder = history_dir(&dir);
    fs::create_dir_all(&folder).expect("create the history");
    fs::write(folder.join("runs.jsonl"), format!("{hook}\n")).expect("write");
    fs::write(
        folder.join("findings.jsonl"),
        format!("{}\n", finding(1, "no-utils", 1)),
    )
    .expect("write");
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(2, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );
    let downloads = dir.join("downloads");

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        section(&text, "Caught before the commit").contains("none has a CI run"),
        "{text}"
    );
}

#[test]
fn counts_that_overflow_do_not_stop_the_report() {
    let dir = dir("overflow");
    let big = u64::MAX;
    write_local(
        &dir,
        &[
            RunSpec::new(1, "cli", "full").counted("huge", (big, big, 0), None),
            RunSpec::new(2, "cli", "full").counted("huge", (big, big, 0), None),
        ],
        &[],
    );

    let output = report(&dir, &[]);

    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn a_name_with_a_character_that_reorders_text_is_a_line_that_fails_its_check() {
    let dir = dir("bidi");
    write_local(
        &dir,
        &[
            RunSpec::new(1, "cli", "full").rule("evil\u{202E}name", 0, None),
            RunSpec::new(2, "cli", "full").rule("fine", 0, None),
        ],
        &[],
    );

    let text = stdout(&report(&dir, &[]));

    assert!(!text.contains('\u{202E}'), "{text}");
    assert!(text.contains("1 lines failed their checks"), "{text}");
}

#[test]
fn a_history_directory_that_is_a_symbolic_link_is_not_followed() {
    let dir = dir("history-link");
    let outside = dir.join("outside");
    write_in(
        &outside,
        &[RunSpec::new(1, "cli", "full").rule("through-link", 0, None)],
        &[],
    );
    fs::create_dir_all(dir.join(".botica")).expect("create .botica");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, dir.join(".botica").join("history")).expect("link");

        let text = stdout(&report(&dir, &[]));

        assert!(!text.contains("through-link"), "{text}");
        assert!(
            text.contains("is a symbolic link or not a directory"),
            "{text}"
        );
    }
}

#[test]
fn a_folder_of_from_that_is_a_symbolic_link_is_named_and_not_read() {
    let dir = dir("link-run");
    let downloads = dir.join("downloads");
    fs::create_dir_all(&downloads).expect("create downloads");
    let outside = dir.join("outside");
    write_in(
        &outside,
        &[RunSpec::new(1, "ci", "full").rule("linked", 0, None)],
        &[],
    );
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, downloads.join("100")).expect("link");

        let text = stdout(&report(
            &dir,
            &["--from", downloads.to_str().expect("UTF-8")],
        ));

        assert!(
            text.contains("is not a folder of a CI run: skipped"),
            "{text}"
        );
        assert!(!text.contains("linked: no finding"), "{text}");
    }
}

#[test]
fn a_source_file_that_does_not_say_trusted_with_a_boolean_is_not_trusted() {
    let dir = dir("source-shape");
    let downloads = dir.join("downloads");
    for (name, body) in [("100", "not json"), ("101", r#"{"trusted":"yes"}"#)] {
        write_downloaded(
            &downloads,
            name,
            true,
            &[
                RunSpec::new(name.parse().expect("number"), "ci", "full").rule(
                    "claims-trust",
                    0,
                    None,
                ),
            ],
            &[],
        );
        fs::write(downloads.join(name).join("source.json"), body).expect("source");
    }

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        !section(&text, "Rules with no findings").contains("claims-trust"),
        "{text}"
    );
    assert!(text.contains("does not parse: not trusted"), "{text}");
}

#[test]
fn the_runs_of_a_role_from_a_source_that_is_not_trusted_are_not_counted() {
    let dir = dir("untrusted-role");
    let run = RunSpec::new(1, "review:security", "reviewed").rule("review:security", 1, None);
    let mut judged = finding(1, "review:security", 1);
    judged["refutation"] = json!("duplicate");
    write_downloaded(&dir.join("downloads"), "100", false, &[run], &[judged]);
    let downloads = dir.join("downloads");

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        !text.contains("Findings of the roles that refute did not confirm"),
        "{text}"
    );
}

#[test]
fn a_finding_that_two_hook_runs_hold_counts_once() {
    let dir = dir("dedup");
    write_local(
        &dir,
        &[
            RunSpec::new(1, "hook", "staged")
                .ago(9000)
                .commit(COMMIT_A)
                .rule("no-utils", 0, Some(HASH)),
            RunSpec::new(2, "hook", "staged")
                .ago(8000)
                .commit(COMMIT_A)
                .rule("no-utils", 0, Some(HASH)),
        ],
        &[finding(1, "no-utils", 5), finding(2, "no-utils", 5)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(3, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[],
    );
    let downloads = dir.join("downloads");

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        section(&text, "Caught before the commit").contains("no-utils: 1\n")
            || section(&text, "Caught before the commit")
                .trim_end()
                .ends_with("no-utils: 1"),
        "{text}"
    );
}

#[test]
fn a_ci_run_is_compared_with_every_hook_run_before_it() {
    let dir = dir("every-hook");
    write_local(
        &dir,
        &[
            RunSpec::new(1, "hook", "staged")
                .ago(9000)
                .commit(COMMIT_A)
                .rule("no-utils", 0, Some(HASH)),
            RunSpec::new(2, "hook", "staged")
                .ago(8000)
                .commit(COMMIT_A)
                .rule("no-utils", 0, Some(HASH)),
        ],
        &[finding(2, "no-utils", 7)],
    );
    write_downloaded(
        &dir.join("downloads"),
        "100",
        true,
        &[RunSpec::new(3, "ci", "diff")
            .ago(3600)
            .head(COMMIT_B)
            .rule("no-utils", 0, Some(HASH))],
        &[finding(3, "no-utils", 7)],
    );
    let downloads = dir.join("downloads");

    let text = stdout(&report(
        &dir,
        &["--from", downloads.to_str().expect("UTF-8")],
    ));

    assert!(
        section(&text, "Reached CI").contains("No finding."),
        "{text}"
    );
}
