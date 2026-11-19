//! Mutation selection through installed recipes and the real risk classifier.

#![cfg(unix)]

mod common;

use std::path::Path;
use std::process::{Command, Output};

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));
    common::stdout(&output).trim().to_owned()
}

fn installed_recipe(recipe: &str, case: &str) -> (std::path::PathBuf, String, String) {
    let dir = common::project_in(
        "mutation-consumers",
        &format!("{recipe}-{case}"),
        Some(
            "[[review.areas]]\nname = 'periphery'\npaths = ['docs/**']\n\
              [[review.areas]]\nname = 'platform'\npaths = ['.github/**']\nowners = ['@test']\n",
        ),
    );
    let installed = common::command(&dir, &["recipe", "install", recipe]);
    assert!(installed.status.success(), "{}", common::stderr(&installed));
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Mutation test"]);
    git(&dir, &["config", "user.email", "mutation@example.com"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Install recipe"]);
    let base = git(&dir, &["rev-parse", "HEAD"]);
    git(&dir, &["switch", "-c", "topic"]);
    match case {
        "low" => common::write(&dir, "docs/note.md", "A changed note.\n"),
        "medium" => common::write(&dir, "src/lib.rs", "pub fn answer() -> u8 { 42 }\n"),
        "unsupported" => common::write_bytes(&dir, "image.png", b"\0binary"),
        _ => unreachable!(),
    }
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Change"]);
    let head = git(&dir, &["rev-parse", "HEAD"]);
    git(&dir, &["update-ref", "refs/pull/42/head", &head]);
    git(&dir, &["remote", "add", "origin", "."]);
    git(&dir, &["switch", "main"]);
    (dir, base, head)
}

fn run_selection(dir: &Path, recipe: &str, base: &str, head: &str) -> (Output, String) {
    use std::os::unix::fs::PermissionsExt;

    let language = recipe.strip_prefix("ci-mutants-").unwrap();
    let workflow =
        std::fs::read_to_string(dir.join(format!(".github/workflows/mutants-{language}.yml")))
            .unwrap();
    let body = workflow
        .split("        run: |\n")
        .nth(1)
        .unwrap()
        .split("\n  mutants:")
        .next()
        .unwrap();
    let script = body
        .lines()
        .map(|line| line.strip_prefix("          ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    let bin = dir.join(".tmp/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let uvx = bin.join("uvx");
    std::fs::write(
        &uvx,
        "#!/bin/sh\nshift\nexec \"$BOTICA_UNDER_TEST\" \"$@\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&uvx, std::fs::Permissions::from_mode(0o755)).unwrap();
    let result_path = dir.join(".tmp/outputs");
    let output = Command::new("bash")
        .args(["-eo", "pipefail", "-c", &script])
        .current_dir(dir)
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .env("BOTICA_UNDER_TEST", env!("CARGO_BIN_EXE_botica"))
        .env("RUNNER_TEMP", dir.join(".tmp"))
        .env("GITHUB_OUTPUT", &result_path)
        .env("BASE_SHA", base)
        .env("HEAD_SHA", head)
        .env("PR_NUMBER", "42")
        .output()
        .unwrap();
    let selected = std::fs::read_to_string(result_path).unwrap_or_default();
    (output, selected)
}

#[cfg(unix)]
#[test]
fn consumer_mutation_selection_does_not_require_role_definitions() {
    for recipe in ["ci-mutants-rust", "ci-mutants-go", "ci-mutants-node"] {
        for (case, expected) in [("low", "run=false\n"), ("medium", "run=true\n")] {
            let (dir, base, head) = installed_recipe(recipe, case);
            assert!(!dir.join(".github/review").exists());
            let (output, selected) = run_selection(&dir, recipe, &base, &head);
            assert!(
                output.status.success(),
                "{recipe}/{case}: {}",
                common::stderr(&output)
            );
            assert_eq!(selected, expected, "{recipe}/{case}");
        }
    }
}

#[cfg(unix)]
#[test]
fn unsupported_consumer_risk_still_blocks_mutation_selection() {
    for recipe in ["ci-mutants-rust", "ci-mutants-go", "ci-mutants-node"] {
        let (dir, base, head) = installed_recipe(recipe, "unsupported");
        let (output, selected) = run_selection(&dir, recipe, &base, &head);
        assert!(!output.status.success(), "{recipe}");
        assert!(selected.is_empty(), "{recipe}: {selected}");
    }
}
