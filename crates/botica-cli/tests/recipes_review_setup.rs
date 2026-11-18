//! A base installation supplies a usable, customizable consumer review policy.

mod common;

use std::path::Path;
use std::process::Command;

const ROLES: [&str; 7] = [
    "spec",
    "security",
    "tests",
    "architecture",
    "contracts",
    "performance",
    "refute",
];

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));
}

fn prepared_consumer() -> std::path::PathBuf {
    let dir = common::project_in("recipe-review", "base", None);
    let recipes = botica_catalog::Recipes::embedded().unwrap();
    let base = recipes.recipe("base").unwrap();
    let names: Vec<&str> = base
        .skills
        .iter()
        .flat_map(|step| step.names.iter().map(String::as_str))
        .collect();
    for role in ROLES.into_iter().chain(["panel"]) {
        assert!(names.contains(&format!("review-{role}").as_str()), "{role}");
    }
    for name in &names {
        common::recipes::install_skill(&dir, name);
    }
    common::recipes::lock_skills(&dir, &names);
    dir
}

#[test]
fn base_installs_review_policy_that_the_real_planner_and_checker_accept() {
    let dir = prepared_consumer();
    let installed = common::recipes::without_npx(&dir, &["install", "base"]);
    assert!(installed.status.success(), "{}", common::stderr(&installed));
    assert!(dir.join(".github/review/README.md").is_file());
    for role in ROLES {
        let definition =
            std::fs::read_to_string(dir.join(format!(".github/review/{role}/REVIEW.md"))).unwrap();
        assert!(definition.contains("covered:"));
        assert!(!definition.contains("@jedzill4"));
        assert!(!definition.contains("botica-core"));
    }
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Review setup test"]);
    git(&dir, &["config", "user.email", "review@example.com"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Install base"]);
    git(&dir, &["switch", "-c", "topic"]);
    common::write(&dir, "note.md", "A changed note.\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Change note"]);
    for command in ["plan", "check"] {
        let output = common::command(
            &dir,
            &[
                "review",
                command,
                "--diff",
                "refs/heads/main",
                "--integration-method",
                "merge",
                "--format",
                "json",
            ],
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(output.status.success(), "{report}");
        assert_eq!(report["status"], "complete");
        let roles = report["roles"].as_array().unwrap();
        assert_eq!(roles.len(), 7);
        assert!(roles
            .iter()
            .all(|role| role["covered"]["status"] == "current"));
        assert!(report["inputs"]["trusted_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|file| file["path"] == ".github/review/README.md"));
    }
    let exported = common::command(&dir, &["rules", "export", "--format", "abide"]);
    assert!(exported.status.success(), "{}", common::stderr(&exported));
    let check = common::recipes::without_npx(&dir, &["check", "base"]);
    assert!(check.status.success(), "{}", common::stdout(&check));
    let repeated = common::recipes::without_npx(&dir, &["install", "base"]);
    assert!(repeated.status.success(), "{}", common::stdout(&repeated));
    let role = dir.join(".github/review/architecture/REVIEW.md");
    let customized = format!(
        "{}\nConsumer-specific invariant.\n",
        std::fs::read_to_string(&role).unwrap()
    );
    std::fs::write(&role, &customized).unwrap();
    let repeated = common::recipes::without_npx(&dir, &["install", "base"]);
    assert_eq!(repeated.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(role).unwrap(), customized);
}
