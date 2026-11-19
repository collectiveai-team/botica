use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("judged")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the working directory");
    dir
}

fn botica(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["lint"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

const NO_UTILS: &str = "{\"version\": \"2.1.0\", \"runs\": [{\"results\": [{\"ruleId\": \
    \"no-utils\", \"message\": {\"text\": \"m\"}, \"locations\": [{\"physicalLocation\": \
    {\"artifactLocation\": {\"uri\": \"utils.py\"}}}]}]}]}";

#[test]
fn a_finding_of_a_deterministic_rule_is_left_out_with_a_warning_and_nothing_is_linted() {
    let dir = workdir("deterministic");
    fs::write(dir.join("judge.sarif"), NO_UTILS).expect("write the SARIF");
    fs::write(dir.join("utils.py"), "x = 1\n").expect("write a file botica would flag");

    let output = botica(&dir, &["--judged", "judge.sarif"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "botica: `no-utils` is not a judgment rule of the catalog: its finding is left out\n"
    );
}

#[test]
fn a_sarif_that_cannot_be_read_or_parsed_fails_the_run() {
    let dir = workdir("broken");
    fs::write(dir.join("broken.sarif"), "not json").expect("write the SARIF");

    for (file, expected) in [
        ("missing.sarif", "cannot read `missing.sarif`"),
        (
            "broken.sarif",
            "the judged findings are neither SARIF nor a report of `abide check --json`",
        ),
    ] {
        let output = botica(&dir, &["--judged", file]);

        assert_eq!(output.status.code(), Some(2), "{file}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{file}: {stderr}");
    }
}

const ABIDE: &str = "{\"root\": \".\", \"sections\": [{\"phase\": \"edit\", \"files\": \
    [\"tests/test_a.py\"], \"modelRules\": 1, \"calls\": 1, \"latencyMs\": 10, \"verdicts\": \
    [{\"ruleId\": \"test-through-interface\", \"probability\": 0.93, \"band\": \"act\"}]}], \
    \"spendUsd\": 0.001, \"all\": false}";

#[test]
fn a_report_of_abide_becomes_sarif_and_fails_the_run_at_the_severity_of_fail_on() {
    let dir = workdir("abide");
    fs::write(
        dir.join("botica.toml"),
        "preview = true\nselect = [\"test-through-interface\"]\n",
    )
    .expect("write the configuration");
    fs::create_dir_all(dir.join("tests")).expect("create tests/");
    fs::write(dir.join("tests/test_a.py"), "def test_a():\n    pass\n").expect("write a test");
    fs::write(dir.join("abide.json"), ABIDE).expect("write the report");

    let sarif = botica(
        &dir,
        &["--judged", "abide.json", "--output-format", "sarif"],
    );
    let gate = botica(&dir, &["--judged", "abide.json", "--fail-on", "warning"]);

    assert_eq!(sarif.status.code(), Some(0));
    let document: serde_json::Value =
        serde_json::from_slice(&sarif.stdout).expect("the output is SARIF");
    let result = &document["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "test-through-interface");
    assert_eq!(
        result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
        "tests/test_a.py"
    );
    assert_eq!(gate.status.code(), Some(1));
}

#[test]
fn with_a_diff_a_judged_finding_stays_only_when_the_diff_changes_its_file() {
    let dir = workdir("diff");
    fs::write(
        dir.join("botica.toml"),
        "preview = true\nselect = [\"test-through-interface\"]\n",
    )
    .expect("write the configuration");
    fs::create_dir_all(dir.join("tests")).expect("create tests/");
    fs::write(dir.join("tests/test_a.py"), "def test_a():\n    pass\n").expect("write a test");
    fs::write(dir.join("abide.json"), ABIDE).expect("write the report");
    let patch = |path: &str| format!("--- a/{path}\n+++ b/{path}\n@@ -1 +1 @@\n-x\n+y\n");
    fs::write(dir.join("touches.diff"), patch("tests/test_a.py")).expect("write a patch");
    fs::write(dir.join("other.diff"), patch("tests/test_b.py")).expect("write a patch");

    let touched = botica(
        &dir,
        &["--judged", "abide.json", "--diff-file", "touches.diff"],
    );
    let other = botica(
        &dir,
        &["--judged", "abide.json", "--diff-file", "other.diff"],
    );

    assert!(String::from_utf8_lossy(&touched.stdout).contains("tests/test_a.py"));
    assert_eq!(String::from_utf8_lossy(&other.stdout), "");
}

fn report(rule: &str, file: &str, probability: &str, band: &str) -> String {
    format!(
        "{{\"root\": \".\", \"sections\": [{{\"phase\": \"edit\", \"files\": [\"{file}\"], \
         \"verdicts\": [{{\"ruleId\": \"{rule}\", \"probability\": {probability}, \
         \"band\": \"{band}\"}}]}}], \"spendUsd\": 0, \"all\": false}}"
    )
}

fn minimal_comments(probability: &str) -> String {
    report("minimal-comments", "a.py", probability, "act")
}

#[test]
fn a_plain_writing_verdict_under_0_85_is_reported_as_info() {
    let dir = workdir("plain-writing-act-at");
    fs::write(
        dir.join("botica.toml"),
        "preview = true\nselect = [\"plain-writing\"]\n",
    )
    .expect("write the configuration");
    fs::write(dir.join("a.md"), "Text.\n").expect("write a file");
    fs::write(
        dir.join("low.json"),
        report("plain-writing", "a.md", "0.84", "act"),
    )
    .expect("write the report");
    fs::write(
        dir.join("high.json"),
        report("plain-writing", "a.md", "0.86", "act"),
    )
    .expect("write the report");

    let low = botica(&dir, &["--judged", "low.json", "--fail-on", "warning"]);
    let high = botica(&dir, &["--judged", "high.json", "--fail-on", "warning"]);

    let text = String::from_utf8_lossy(&low.stdout);
    assert_eq!(low.status.code(), Some(0), "{text}");
    assert!(text.contains("a.md:1:1: info [plain-writing]"), "{text}");
    assert_eq!(high.status.code(), Some(1));
}

#[test]
fn a_verdict_in_the_band_act_under_the_act_at_of_its_rule_is_reported_as_info() {
    let dir = workdir("act-at");
    fs::write(
        dir.join("botica.toml"),
        "preview = true\nselect = [\"minimal-comments\"]\n",
    )
    .expect("write the configuration");
    fs::write(dir.join("a.py"), "# x is one\nx = 1\n").expect("write a file");
    fs::write(dir.join("low.json"), minimal_comments("0.81")).expect("write the report");
    fs::write(dir.join("high.json"), minimal_comments("0.9")).expect("write the report");

    let low = botica(&dir, &["--judged", "low.json", "--fail-on", "warning"]);
    let high = botica(&dir, &["--judged", "high.json", "--fail-on", "warning"]);

    let text = String::from_utf8_lossy(&low.stdout);
    assert_eq!(low.status.code(), Some(0), "{text}");
    assert!(
        text.contains("a.py:1:1: info [minimal-comments]")
            && text.contains("probability 0.81, under the 0.85 that the rule reports at"),
        "{text}"
    );
    assert_eq!(high.status.code(), Some(1));
}

fn plain_writing_runs(name: &str, runs: &[(&str, &str)]) -> (Output, String) {
    let dir = workdir(name);
    fs::write(
        dir.join("botica.toml"),
        "preview = true\nselect = [\"plain-writing\"]\n",
    )
    .expect("write the configuration");
    fs::write(dir.join("a.md"), "Text.\n").expect("write a file");
    let mut args = Vec::new();
    for (index, (probability, band)) in runs.iter().enumerate() {
        let file = format!("run{index}.json");
        fs::write(
            dir.join(&file),
            report("plain-writing", "a.md", probability, band),
        )
        .expect("write the report");
        args.push("--judged".to_owned());
        args.push(file);
    }
    args.extend(["--fail-on".to_owned(), "warning".to_owned()]);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = botica(&dir, &args);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, text)
}

#[test]
fn a_finding_that_a_majority_of_the_runs_give_keeps_its_severity() {
    let (output, text) = plain_writing_runs(
        "majority",
        &[("0.90", "act"), ("0.88", "act"), ("0.70", "flag")],
    );

    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("a.md:1:1: warning [plain-writing]")
            && text.contains("2 of 3 runs of abide see the change break it; probabilities 0.90 act, 0.88 act, 0.70"),
        "{text}"
    );
}

#[test]
fn a_finding_that_a_minority_of_the_runs_give_is_info() {
    for (name, runs) in [
        (
            "minority",
            [("0.90", "act"), ("0.70", "flag"), ("0.50", "clear")],
        ),
        (
            "under-act-at",
            [("0.90", "act"), ("0.84", "act"), ("0.83", "act")],
        ),
    ] {
        let (output, text) = plain_writing_runs(name, &runs);

        assert_eq!(output.status.code(), Some(0), "{name}: {text}");
        assert!(
            text.contains("a.md:1:1: info [plain-writing]"),
            "{name}: {text}"
        );
        assert!(text.contains("1 of 3 runs"), "{name}: {text}");
    }
}

#[test]
fn a_file_that_one_run_leaves_clear_needs_a_majority_of_all_the_runs() {
    let (output, text) = plain_writing_runs(
        "two-of-four",
        &[
            ("0.90", "act"),
            ("0.90", "act"),
            ("0.10", "clear"),
            ("0.10", "clear"),
        ],
    );

    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(text.contains("2 of 4 runs"), "{text}");
}

#[test]
fn judged_findings_take_no_paths_and_no_extension() {
    let dir = workdir("conflicts");
    fs::write(dir.join("judge.sarif"), NO_UTILS).expect("write the SARIF");

    for args in [
        ["--judged", "judge.sarif", "src"].as_slice(),
        ["--judged", "judge.sarif", "--ext", "x"].as_slice(),
    ] {
        assert_eq!(botica(&dir, args).status.code(), Some(2), "{args:?}");
    }
}
