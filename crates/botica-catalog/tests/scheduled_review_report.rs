//! The `run:` bodies of the job `history-report` of `.github/workflows/scheduled-review.yml`, run
//! with bash as the runner runs them. `gh` is a script that answers from files a test writes and
//! logs its arguments; `jq`, `unzip`, `zip`, `awk` and `git` are the real ones. A test skips itself
//! without them, unless `BOTICA_REQUIRE_WORKFLOW_LINTERS` is set (CI).

mod common;
#[path = "scheduled_review_report/helpers.rs"]
mod helpers;
use helpers::*;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::workflows::{repository_workflow, steps};

const TITLE: &str = "Weekly review report";

fn step(name: &str) -> String {
    let workflow = repository_workflow("scheduled-review.yml");
    steps(&workflow["jobs"]["history-report"])
        .iter()
        .find(|s| s["name"].as_str() == Some(name))
        .and_then(|s| s["run"].as_str())
        .unwrap_or_else(|| panic!("no step named {name}"))
        .to_owned()
}

fn tools_installed() -> bool {
    let found = ["jq", "unzip", "zip", "awk", "git"].iter().all(|tool| {
        Command::new("which")
            .arg(tool)
            .output()
            .is_ok_and(|o| o.status.success())
    });
    assert!(
        found || std::env::var_os("BOTICA_REQUIRE_WORKFLOW_LINTERS").is_none(),
        "BOTICA_REQUIRE_WORKFLOW_LINTERS is set but jq, unzip, zip, awk or git is missing"
    );
    found
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn read(root: &Path, file: &str) -> String {
    fs::read_to_string(root.join(file)).unwrap_or_default()
}

fn items(prefix: &str, count: usize) -> String {
    (1..=count)
        .map(|n| format!("- `{prefix}-{n}`: no finding in 3 runs\n"))
        .collect()
}

fn cap(name: &str, report: &str) -> (PathBuf, Output) {
    let root = scratch(name);
    fake_gh(&root);
    fs::write(root.join("report.md"), report).expect("the report");
    fs::write(
        root.join("download-summary.json"),
        r#"{"status":"complete"}"#,
    )
    .unwrap();
    let output = bash(&root, &step("Cap the findings"), &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    (root, output)
}

#[test]
fn a_section_with_more_than_20_items_keeps_20_and_says_how_many_it_left_out() {
    if !tools_installed() {
        return;
    }
    let report = format!(
        "# botica history report\n\n## Runs read\n\n{}\n## Rules with no findings\n\nText.\n{}\n## Rules with many suppressions\n\n- `noisy`: 8 suppressed, 2 reported (80%)\n",
        items("read", 25),
        items("quiet", 23)
    );

    let (root, _) = cap("report-cap", &report);

    let issue = read(&root, "issue.md");
    assert_eq!(
        issue.matches("no finding in 3 runs").count(),
        25 + 19,
        "{issue}"
    );
    assert!(
        issue.contains("omitted 4 beyond the global limit"),
        "{issue}"
    );
    assert!(issue.contains("`quiet-19`"), "{issue}");
    assert!(!issue.contains("`quiet-20`"), "{issue}");
    assert_eq!(issue.matches("<!-- botica:history-item:").count(), 20);
    assert!(issue.contains("`noisy`"), "{issue}");
    assert!(issue.contains("Run: https://example.com/run/1"), "{issue}");
    assert!(read(&root, "output.txt").starts_with("findings=24\n"));
}

#[test]
fn a_report_with_nothing_to_decide_counts_no_finding() {
    if !tools_installed() {
        return;
    }
    let report = "# botica history report\n\n## Caught before the commit\n\n- `no-utils`: 4\n\n\
                  ## Rules with no findings\n\nNone.\n\n## Rules with many suppressions\n\nNone.\n";

    let (root, _) = cap("report-cap-none", report);

    assert!(read(&root, "output.txt").starts_with("findings=0\n"));
    assert!(read(&root, "issue.md").contains("`no-utils`: 4"));
}

fn publish(name: &str, issues: &str, findings: &str) -> (PathBuf, Output) {
    let root = scratch(name);
    fake_gh(&root);
    fs::write(root.join("issue.md"), "# the report\n$(touch pwned) `x`\n").expect("the issue");
    fs::write(root.join("answers/issues.json"), issues).expect("the issues");
    let issues: serde_json::Value = serde_json::from_str(issues).unwrap();
    let number = issues
        .as_array()
        .unwrap()
        .iter()
        .find(|issue| issue["title"] == TITLE)
        .and_then(|issue| issue["number"].as_u64())
        .map(|number| number.to_string())
        .unwrap_or_default();
    let output = bash(
        &root,
        &step("Open or update the issue"),
        &[("FINDINGS", findings), ("ISSUE_NUMBER", &number)],
    );
    (root, output)
}

#[test]
fn an_open_issue_with_the_title_is_edited_and_no_other_is_opened() {
    if !tools_installed() {
        return;
    }
    let issues = format!(
        r#"[{{"number": 7, "title": "{TITLE} (old)"}}, {{"number": 9, "title": "{TITLE}"}}]"#
    );

    let (root, output) = publish("report-edit", &issues, "0");

    assert!(output.status.success(), "{}", stderr(&output));
    let log = read(&root, "gh.log");
    assert!(log.contains("issue edit 9 --body-file"), "{log}");
    assert!(!log.contains("issue create"), "{log}");
    assert!(!root.join("pwned").exists(), "the report ran as a command");
}

#[test]
fn with_no_open_issue_a_report_with_findings_opens_one_and_an_empty_report_opens_none() {
    if !tools_installed() {
        return;
    }
    let other = r#"[{"number": 3, "title": "Something else"}]"#;

    let (root, output) = publish("report-create", other, "2");
    let (quiet, quiet_output) = publish("report-none", "[]", "0");

    assert!(output.status.success(), "{}", stderr(&output));
    let log = read(&root, "gh.log");
    assert!(
        log.contains(&format!("issue create --title {TITLE} --body-file")),
        "{log}"
    );
    assert!(!log.contains("issue edit"), "{log}");
    assert!(quiet_output.status.success(), "{}", stderr(&quiet_output));
    let log = read(&quiet, "gh.log");
    assert!(
        !log.contains("issue create") && !log.contains("issue edit"),
        "{log}"
    );
    assert!(read(&quiet, "summary.md").contains("no issue was opened"));
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "{args:?}: {}", stderr(&output));
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn answer(root: &Path, key: &str, text: &str) {
    fs::write(root.join("answers").join(key), text).expect("an answer");
}

fn zip_of(root: &Path, name: &str, files: &[(&str, &str)]) -> String {
    let dir = root.join(format!("zip-{name}"));
    for (path, text) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().expect("a parent")).expect("the folder");
        fs::write(path, text).expect("the member");
    }
    let archive = root.join(format!("{name}.zip"));
    let mut zip = Command::new("zip");
    zip.args(["-q", "-r"])
        .arg(&archive)
        .arg(".")
        .current_dir(&dir);
    assert!(zip.status().expect("zip runs").success());
    archive.display().to_string()
}

fn artifact(id: u64, name: &str, expired: bool, size: u64) -> serde_json::Value {
    serde_json::json!({"id": id, "name": name, "expired": expired, "size_in_bytes": size})
}

#[test]
fn only_a_run_of_this_repository_on_main_with_a_botica_artifact_is_downloaded_and_trusted() {
    if !tools_installed() {
        return;
    }
    let root = scratch("report-download");
    fake_gh(&root);
    git(&root, &["init", "--quiet", "-b", "main"]);
    git(&root, &["commit", "--quiet", "--allow-empty", "-m", "one"]);
    let on_main = git(&root, &["rev-parse", "HEAD"]);
    git(&root, &["checkout", "--quiet", "-b", "side"]);
    git(&root, &["commit", "--quiet", "--allow-empty", "-m", "side"]);
    let off_main = git(&root, &["rev-parse", "HEAD"]);
    git(&root, &["checkout", "--quiet", "main"]);
    let runs = |entries: &[(u64, &str, &str, &str)]| {
        let runs: Vec<_> = entries
            .iter()
            .map(|(id, event, repo, sha)| {
                serde_json::json!({"id": id, "event": event, "head_sha": sha,
                    "head_repository": {"full_name": repo}})
            })
            .collect();
        serde_json::json!({"workflow_runs": runs}).to_string()
    };
    answer(
        &root,
        "repos_owner_repo_actions_workflows_ci.yml_runs",
        &runs(&[
            (100, "push", "owner/repo", &on_main),
            (101, "pull_request", "owner/repo", &on_main),
            (102, "push", "fork/repo", &on_main),
            (103, "push", "owner/repo", &off_main),
            (104, "workflow_dispatch", "owner/repo", &on_main),
        ]),
    );
    answer(
        &root,
        "repos_owner_repo_actions_workflows_scheduled-review.yml_runs",
        &runs(&[]),
    );
    let artifacts =
        |list: Vec<serde_json::Value>| serde_json::json!({"artifacts": list}).to_string();
    answer(
        &root,
        "repos_owner_repo_actions_runs_100_artifacts",
        &artifacts(vec![
            artifact(1, "botica-history-lint", false, 1000),
            artifact(2, "botica-mutants", false, 1000),
            artifact(5, "botica-history-../x", false, 1000),
        ]),
    );
    answer(
        &root,
        "repos_owner_repo_actions_runs_104_artifacts",
        &artifacts(vec![artifact(6, "botica-history-lint", false, 1000)]),
    );
    for id in [101, 102, 103] {
        answer(
            &root,
            &format!("repos_owner_repo_actions_runs_{id}_artifacts"),
            &artifacts(vec![artifact(9, "botica-history-lint", false, 1000)]),
        );
    }
    let archive = zip_of(
        &root,
        "a",
        &[
            (".botica/history/runs.jsonl", "{\"run\":1}\n"),
            (".botica/history/findings.jsonl", "{\"finding\":1}\n"),
            (".botica/history/source.json", "{\"trusted\": true}\n"),
            ("other.txt", "ignored\n"),
        ],
    );
    fs::copy(
        &archive,
        root.join("answers/repos_owner_repo_actions_artifacts_1_zip"),
    )
    .expect("zip");
    fs::copy(
        &archive,
        root.join("answers/repos_owner_repo_actions_artifacts_6_zip"),
    )
    .expect("zip");
    let workdir = root.join("work");
    git(&root, &["clone", "--quiet", ".", "work"]);
    let temp = root.join("temp");
    fs::create_dir_all(&temp).expect("the temp folder");

    let output = bash_in(
        &root,
        &workdir,
        &step("Download the history of CI"),
        &[
            ("RUNNER_TEMP", temp.to_str().expect("UTF-8")),
            ("GH_TOKEN", "x"),
        ],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let dir = temp.join("ci-history");
    let mut ids: Vec<String> = fs::read_dir(&dir)
        .expect("the downloads")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    ids.sort();
    assert_eq!(ids, ["100", "104"], "{}", read(&root, "gh.log"));
    let folder = dir.join("100/1");
    assert_eq!(
        fs::read_to_string(folder.join("runs.jsonl")).expect("runs"),
        "{\"run\":1}\n"
    );
    assert_eq!(
        fs::read_to_string(folder.join("findings.jsonl")).expect("findings"),
        "{\"finding\":1}\n"
    );
    assert_eq!(
        fs::read_dir(&folder).expect("the folder").count(),
        2,
        "only the two files"
    );
    let source: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(dir.join("100/source.json")).expect("source.json"),
    )
    .expect("JSON");
    assert_eq!(source["trusted"], true);
    assert_eq!(source["head_sha"].as_str(), Some(on_main.as_str()));
    assert!(read(&root, "summary.md").contains("\"status\": \"complete\""));
}
