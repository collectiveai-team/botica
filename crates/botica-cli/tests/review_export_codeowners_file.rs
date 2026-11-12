//! `botica review export --format codeowners`: the file it writes or checks, and what it refuses.

mod common;

use std::fs;

use common::codeowners::{entry, export, printed, project, PLATFORM};
use common::{command, stderr, stdout, write};

#[test]
fn the_default_file_is_written_under_the_root_and_check_passes_after_it() {
    let dir = project("default-file", PLATFORM);
    write(&dir, "sub/keep.txt", "");

    let output = command(
        &dir.join("sub"),
        &["review", "export", "--format", "codeowners"],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let written = fs::read_to_string(dir.join(".github/CODEOWNERS")).expect("the file");
    assert_eq!(written, printed(&dir, &[]));
    let check = export(&dir, &["--check"]);
    assert!(check.status.success(), "{}", stderr(&check));
}

#[test]
fn check_fails_on_a_file_that_differs_or_is_missing_and_writes_nothing() {
    let dir = project("check-differs", PLATFORM);

    let missing = export(&dir, &["--check", "--host", "gitlab"]);
    write(&dir, "owners.txt", "old\n");
    let differs = export(&dir, &["--check", "--output", "owners.txt"]);

    assert_eq!(missing.status.code(), Some(2));
    assert!(
        stderr(&missing).contains(
            ".gitlab/CODEOWNERS` differs from the areas of the configuration: run `botica review \
             export --format codeowners --host gitlab`"
        ),
        "{}",
        stderr(&missing)
    );
    assert!(!dir.join(".gitlab/CODEOWNERS").exists());
    assert!(
        stderr(&differs).contains("--host github --output owners.txt`"),
        "{}",
        stderr(&differs)
    );
    assert_eq!(fs::read_to_string(dir.join("owners.txt")).unwrap(), "old\n");
}

#[test]
fn gitlab_refuses_the_default_file_while_a_codeowners_file_that_it_reads_first_exists() {
    for first in ["CODEOWNERS", "docs/CODEOWNERS"] {
        let dir = project(&format!("gitlab-shadowed-{}", first.replace('/', "-")), PLATFORM);
        write(&dir, first, "* @someone\n");

        let output = export(&dir, &["--host", "gitlab"]);
        let given = export(&dir, &["--host", "gitlab", "--output", "owners.txt"]);

        assert_eq!(output.status.code(), Some(2), "{first}");
        assert!(
            stderr(&output).contains(&format!(
                "GitLab reads `{first}` and not `.gitlab/CODEOWNERS`"
            )),
            "{}",
            stderr(&output)
        );
        assert!(!dir.join(".gitlab/CODEOWNERS").exists());
        assert!(given.status.success(), "{}", stderr(&given));
        assert!(dir.join("owners.txt").is_file());
    }
}

#[test]
fn a_configuration_below_the_root_of_the_repository_needs_an_output() {
    let dir = project("below-root", "");
    write(&dir, "app/botica.toml", PLATFORM);

    let output = export(&dir.join("app"), &[]);
    let given = export(&dir.join("app"), &["--output", "-"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("which is not the root of the git repository"),
        "{}",
        stderr(&output)
    );
    assert!(!dir.join("app/.github/CODEOWNERS").exists());
    assert!(given.status.success(), "{}", stderr(&given));
}

#[test]
fn gitlab_takes_a_nested_group_that_github_refuses() {
    let dir = project(
        "nested-group-gitlab",
        &entry("platform", &["ci/**"], &["@org/group/team"], ""),
    );

    let text = printed(&dir, &["--host", "gitlab"]);

    assert!(text.contains("/ci/ @org/group/team\n"), "{text}");
}

#[test]
fn an_export_that_cannot_be_written_faithfully_fails() {
    let cases = [
        (
            "no-areas",
            String::new(),
            "github",
            "has no [[review.areas]]",
        ),
        (
            "class",
            format!("{PLATFORM}{}", entry("core", &["src/[ab]/**"], &["@o"], "")),
            "github",
            "`src/[ab]/**` has a character class",
        ),
        (
            "nested-group",
            entry("platform", &["ci/**"], &["@org/group/team"], ""),
            "github",
            "`@org/group/team` is a group of GitLab",
        ),
        (
            "same-line",
            format!(
                "{PLATFORM}{}{}",
                entry("core", &["src/**"], &["@a"], ""),
                entry("core", &["/src/"], &["@b"], "")
            ),
            "gitlab",
            "`/src/` comes from a `core` entry and a `core` entry with other owners",
        ),
        (
            "same-line-two-areas",
            format!(
                "{PLATFORM}{}{}",
                entry("core", &["src/**"], &["@a"], "tier = \"high\"\n"),
                entry("critical", &["/src/"], &["@b"], "")
            ),
            "github",
            "`/src/` comes from a `core` entry and a `critical` entry with other owners",
        ),
        (
            "space",
            format!("{PLATFORM}{}", entry("core", &["my dir/**"], &["@o"], "")),
            "github",
            "`my dir/**` has a space",
        ),
        (
            "escape",
            format!("{PLATFORM}{}", entry("core", &["a\\\\*b"], &["@o"], "")),
            "github",
            "`a\\*b` has an escape",
        ),
        (
            "hash",
            format!("{PLATFORM}{}", entry("core", &["lib/a#b/**"], &["@o"], "")),
            "github",
            "`lib/a#b/**` has a `#`",
        ),
        (
            "not-ascii",
            format!("{PLATFORM}{}", entry("core", &["docs/año/**"], &["@o"], "")),
            "gitlab",
            "`docs/año/**` has a character that is not ASCII",
        ),
        (
            "three-stars",
            format!("{PLATFORM}{}", entry("core", &["a/***"], &["@o"], "")),
            "github",
            "`a/***` has `***`",
        ),
        (
            "empty-alternative",
            format!("{PLATFORM}{}", entry("core", &["{src,}"], &["@o"], "")),
            "github",
            "`{src,}` has an empty alternative",
        ),
    ];
    for (name, config, host, message) in cases {
        let dir = project(&format!("refused-{name}"), &config);

        let output = export(&dir, &["--output", "-", "--host", host]);

        assert_eq!(output.status.code(), Some(2), "{name}: {}", stderr(&output));
        assert!(
            stderr(&output).contains(message),
            "{name}: {}",
            stderr(&output)
        );
        assert!(stdout(&output).is_empty(), "{name}");
    }
}

#[test]
fn check_with_the_standard_output_is_refused() {
    let dir = project("check-stdout", PLATFORM);

    let output = export(&dir, &["--check", "--output", "-"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("leave out `--output -`"),
        "{}",
        stderr(&output)
    );
}
