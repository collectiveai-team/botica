//! The step `Judge the diff` of `.github/workflows/abide.yml`, run with bash as the runner runs it.
//! `npx` is a script that prints one report of abide per call from files a test writes and counts
//! the calls; `git` and botica are the real ones.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The `run: |` block of the step `name`, without its indentation.
fn step(name: &str) -> String {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/abide.yml"),
    )
    .expect("the workflow");
    let start = text
        .find(&format!("      - name: {name}\n        run: |\n"))
        .unwrap_or_else(|| panic!("no step named {name}"));
    text[start..]
        .lines()
        .skip(2)
        .take_while(|line| line.is_empty() || line.starts_with("          "))
        .map(|line| line.get(10..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

fn report(probability: &str, band: &str) -> String {
    format!(
        "{{\"root\": \".\", \"sections\": [{{\"phase\": \"edit\", \"files\": [\"a.md\"], \
         \"verdicts\": [{{\"ruleId\": \"plain-writing\", \"probability\": {probability}, \
         \"band\": \"{band}\"}}]}}], \"spendUsd\": 0, \"all\": false}}"
    )
}

/// A repository with one commit on top of its base, and an `npx` that answers `answers` in order.
fn repository(name: &str, answers: &[(&str, &str)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("abide-workflow")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("bin")).expect("the scratch folder");
    fs::create_dir_all(root.join("tmp")).expect("the runner temp");
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@example.com"]);
    git(&["config", "user.name", "t"]);
    fs::write(
        root.join("botica.toml"),
        "preview = true\nselect = [\"plain-writing\"]\n",
    )
    .unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "base"]);
    fs::write(root.join("a.md"), "Text.\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "change"]);
    for (index, (probability, band)) in answers.iter().enumerate() {
        fs::write(
            root.join(format!("answer-{}.json", index + 1)),
            report(probability, band),
        )
        .unwrap();
    }
    let npx = root.join("bin/npx");
    fs::write(
        &npx,
        "#!/bin/bash\ncalls=$(( $(cat \"$ROOT/calls\" 2>/dev/null || echo 0) + 1 ))\n\
         echo \"$calls\" > \"$ROOT/calls\"\ncat \"$ROOT/answer-$calls.json\"\n\
         grep -q '\"band\": \"act\"' \"$ROOT/answer-$calls.json\" && exit 1\nexit 0\n",
    )
    .unwrap();
    Command::new("chmod").arg("+x").arg(&npx).status().unwrap();
    root
}

fn run(root: &Path, script: &str) -> Output {
    let base = Command::new("git")
        .args(["rev-parse", "HEAD~1"])
        .current_dir(root)
        .output()
        .unwrap();
    let old = std::env::var("PATH").unwrap_or_default();
    Command::new("bash")
        .args(["--noprofile", "--norc", "-e", "-c", script])
        .current_dir(root)
        .env("PATH", format!("{}:{old}", root.join("bin").display()))
        .env("ROOT", root)
        .env("RUNNER_TEMP", root.join("tmp"))
        .env("TYPESAFE_AI_API_KEY", "test")
        .env("BASE_SHA", String::from_utf8_lossy(&base.stdout).trim())
        .env("BOTICA", env!("CARGO_BIN_EXE_botica"))
        .output()
        .expect("bash runs")
}

fn calls(root: &Path) -> String {
    fs::read_to_string(root.join("calls"))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn judge_and_fail(name: &str, answers: &[(&str, &str)]) -> (PathBuf, Output, Output) {
    let root = repository(name, answers);
    let judged = run(&root, &step("Judge the diff"));
    assert!(
        judged.status.success(),
        "{}",
        String::from_utf8_lossy(&judged.stderr)
    );
    let failed = run(&root, &step("Fail on the findings"));
    (root, judged, failed)
}

#[test]
fn a_first_run_with_no_finding_calls_abide_once() {
    let (root, _, failed) = judge_and_fail("clear", &[("0.10", "clear")]);

    assert_eq!(calls(&root), "1");
    assert!(failed.status.success());
}

#[test]
fn a_finding_that_two_of_three_runs_give_fails_the_job() {
    let (root, _, failed) = judge_and_fail(
        "two-of-three",
        &[("0.90", "act"), ("0.10", "clear"), ("0.88", "act")],
    );

    assert_eq!(calls(&root), "3");
    assert_eq!(failed.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&failed.stdout).contains("2 of 3 runs"),
        "{}",
        String::from_utf8_lossy(&failed.stdout)
    );
}

#[test]
fn a_finding_that_one_of_three_runs_gives_passes_the_job() {
    let (root, _, failed) = judge_and_fail(
        "one-of-three",
        &[("0.90", "act"), ("0.10", "clear"), ("0.70", "flag")],
    );

    assert_eq!(calls(&root), "3");
    assert!(
        failed.status.success(),
        "{}",
        String::from_utf8_lossy(&failed.stdout)
    );
}
