//! `botica lint --commit-msg`, `--commits` and `--pr-title` (ADR 0010): the rules of scope `commit`
//! over commit messages, and no file.

mod common;

use common::{command, project_in, stderr, stdout, write};
use std::path::Path;
use std::process::Command;

const BOTH: &str = "preview = true\nselect = [\"conventional-commits\", \"no-ai-coauthorship\"]\n";

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?}");
}

fn lint(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let mut all = vec!["lint"];
    all.extend(args);
    let output = command(dir, &all);
    (output.status.code(), stdout(&output))
}

#[test]
fn commits_reports_each_commit_of_the_range_by_its_short_sha() {
    let dir = project_in("lint-messages", "commits", Some(BOTH));
    std::fs::remove_dir_all(dir.join(".git")).expect("remove the stand-in .git");
    git(&dir, &["init", "--quiet"]);
    write(&dir, "a.py", "print(1)\n");
    git(&dir, &["add", "--all"]);
    git(&dir, &["commit", "--quiet", "-m", "feat: the first commit"]);
    git(
        &dir,
        &[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "Second commit\n\nCo-authored-by: Claude <noreply@anthropic.com>",
        ],
    );
    let sha = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(&dir)
            .output()
            .expect("run git")
            .stdout,
    )
    .expect("UTF-8");
    let sha = sha.trim();

    let (code, text) = lint(&dir, &["--commits", "HEAD~1..HEAD"]);

    assert_eq!(code, Some(1), "{text}");
    let places: Vec<&str> = text
        .lines()
        .map(|line| line.split(": ").next().unwrap_or(""))
        .collect();
    assert_eq!(
        places,
        [format!("commit {sha}:1:1"), format!("commit {sha}:3:1")]
    );
    assert!(
        text.contains("[conventional-commits] The subject `Second commit`"),
        "{text}"
    );
    assert!(!text.contains("a.py"), "{text}");
}

#[test]
fn commit_msg_reads_the_file_of_the_hook_without_its_comments() {
    let dir = project_in("lint-messages", "commit-msg", Some(BOTH));
    write(
        &dir,
        "COMMIT_EDITMSG",
        "fix: drop the flag\n# Co-authored-by: Claude\n\
         # ------------------------ >8 ------------------------\nGenerated with Claude Code\n",
    );

    assert_eq!(
        lint(&dir, &["--commit-msg", "COMMIT_EDITMSG"]),
        (Some(0), String::new())
    );
}

#[test]
fn pr_title_is_a_message_of_one_line_and_the_configuration_sets_the_options_and_severity() {
    let config = format!(
        "{BOTH}[rules.conventional-commits]\ntypes = [\"feat\", \"wip\"]\nseverity = \"warning\"\n"
    );
    let dir = project_in("lint-messages", "pr-title", Some(&config));

    assert_eq!(
        lint(&dir, &["--pr-title=wip: half done"]),
        (Some(0), String::new())
    );
    let (code, text) = lint(&dir, &["--pr-title=fix: done"]);
    assert_eq!(code, Some(0), "{text}");
    assert!(
        text.starts_with("pull request title:1:1: warning [conventional-commits]"),
        "{text}"
    );
}

#[test]
fn a_run_reads_messages_or_files_and_not_both() {
    let dir = project_in("lint-messages", "conflicts", Some(BOTH));
    write(&dir, "m.txt", "feat: x\n");

    for args in [
        ["--commit-msg", "m.txt", "--", "a.py"].as_slice(),
        ["--pr-title=feat: x", "--commits", "HEAD"].as_slice(),
        ["--pr-title=feat: x", "--staged"].as_slice(),
        ["--pr-title=feat: x", "--judged", "j.json"].as_slice(),
    ] {
        let mut all = vec!["lint"];
        all.extend(args);
        let output = command(&dir, &all);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            stderr(&output)
        );
    }
}
