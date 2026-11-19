use super::*;

pub(super) fn scratch(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("bin")).expect("the scratch folder");
    root
}

fn write_executable(path: &Path, text: &str) {
    fs::write(path, text).expect("the script");
    Command::new("chmod")
        .arg("+x")
        .arg(path)
        .status()
        .expect("chmod runs");
}

pub(super) fn fake_gh(root: &Path) {
    fs::create_dir_all(root.join("answers")).expect("the answers");
    fs::write(root.join("answers/repos_owner_repo_actions_workflows"),
        r#"{"workflows":[{"path":".github/workflows/ci.yml"},{"path":".github/workflows/scheduled-review.yml"},{"path":".github/workflows/review.yml"},{"path":".github/workflows/review-shadow.yml"},{"path":".github/workflows/review-feedback.yml"}]}"#).unwrap();
    for workflow in ["review-shadow.yml", "review-feedback.yml"] {
        fs::write(
            root.join(format!(
                "answers/repos_owner_repo_actions_workflows_{workflow}_runs"
            )),
            r#"{"workflow_runs":[]}"#,
        )
        .unwrap();
    }
    fs::write(
        root.join("answers/repos_owner_repo_actions_workflows_review.yml_runs"),
        r#"{"workflow_runs":[]}"#,
    )
    .unwrap();
    fs::write(root.join("answers/repos_owner_repo_issues"), "[]").unwrap();
    write_executable(
        &root.join("bin/gh"),
        r#"#!/bin/bash
echo "$*" >> "$ROOT/gh.log"
case "$1 $2" in
  "api "*)
    for arg in "$@"; do endpoint=$arg; done
    key=$(echo "${endpoint%%\?*}" | tr '/' '_')
    cat "$ROOT/answers/$key" ;;
  "issue list") cat "$ROOT/answers/issues.json" ;;
  *) ;;
esac
"#,
    );
}

pub(super) fn bash(root: &Path, script: &str, env: &[(&str, &str)]) -> Output {
    bash_in(root, root, script, env)
}

pub(super) fn bash_in(root: &Path, cwd: &Path, script: &str, env: &[(&str, &str)]) -> Output {
    fs::create_dir_all(cwd.join("scripts")).unwrap();
    for (name, text) in [
        (
            "review_history_download.py",
            include_str!("../../../../scripts/review_history_download.py"),
        ),
        (
            "review_weekly_report.py",
            include_str!("../../../../scripts/review_weekly_report.py"),
        ),
    ] {
        fs::write(cwd.join("scripts").join(name), text).unwrap();
    }
    let old = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}:{old}", root.join("bin").display());
    Command::new("bash")
        .args(["--noprofile", "--norc", "-eo", "pipefail", "-c", script])
        .current_dir(cwd)
        .env("PATH", path)
        .env("ROOT", root)
        .env("RUNNER_TEMP", root)
        .env("GITHUB_OUTPUT", root.join("output.txt"))
        .env("GITHUB_STEP_SUMMARY", root.join("summary.md"))
        .env("REPORT_TITLE", TITLE)
        .env("REPORT_MAX_FINDINGS", "20")
        .env("HISTORY_DAYS", "28")
        .env("GH_REPO", "owner/repo")
        .env("BRANCH", "main")
        .env("RUN_URL", "https://example.com/run/1")
        .env("ISSUE_NUMBER", "")
        .envs(env.iter().copied())
        .output()
        .expect("bash runs")
}
