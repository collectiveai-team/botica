//! A glob of `periphery` or of `generated` that can select a path that a person has to review fails
//! to load, however narrow the glob is (ADR 0018 decision 1 and decision 6). The overlap is decided
//! on the globs, and a glob that this check does not follow exactly is read as selecting everything.

mod common;

use std::time::{Duration, Instant};

use botica_config::Config;
use common::review::{accepted, catalog, problems, with_platform};

fn periphery(glob: &str) -> String {
    with_platform(&format!(
        "[[review.areas]]\nname = \"periphery\"\npaths = [\"{glob}\"]\n"
    ))
}

fn generated(glob: &str) -> String {
    format!(
        "[review]\ngenerated = [\"{glob}\"]\n\n{}",
        with_platform(
            "[[review.areas]]\nname = \"critical\"\npaths = [\"db/migrations/**\"]\nowners = [\"@o\"]\n"
        )
    )
}

/// The message of the load error of `text`, or a failure that says the configuration loads.
fn refusal(text: &str, what: &str) -> String {
    let config = Config::from_botica_toml(text).expect("parses");
    match config.validate(&catalog()) {
        Err(error) => error.to_string(),
        Ok(()) => panic!("`{what}` loads"),
    }
}

#[test]
fn a_periphery_glob_that_selects_some_forced_files_fails_however_narrow_it_is() {
    for glob in [
        ".github/workflows/*.yml",
        ".github/workflows/ci.yml",
        ".github/review/*.md",
        ".github/**/*.yml",
        "docs/**/AGENTS.md",
        "skills/**/SKILL.md",
        "skills/*/SKILL.md",
        "**/INVARIANTS.md",
        "*.md",
        "/botica.toml",
        ".abide/rubric.json",
        ".claude/*.json",
        ".claude/settings.json",
        "pyproject.toml",
        "/pyproject.toml",
        "{docs,.github}/**",
        "docs/[A-Z]*/AGENTS.md",
        "/.gith*",
        "/.gith*/",
        "/docs/s*",
        "/.github[/]workflows/ci.yml",
        "/prek.t[]o]ml",
        "/prek.t[!]x]oml",
        "{a\\\\{,.github}/workflows/ci.yml",
        "{x[{],.github}/workflows/ci.yml",
        "/docs/.claude/**",
        "/docs/.cl*/**",
        "/docs/**/.cla[u]de/**",
        "docs/**/*.png",
        "/docs/.claude",
        "/docs/**/.claude",
    ] {
        let message = refusal(&periphery(glob), glob);

        assert!(
            message.contains("review.areas[1].paths[0]") && message.contains("platform"),
            "`{glob}`: {message}"
        );
    }
}

#[test]
fn a_periphery_glob_that_selects_no_forced_file_loads() {
    for glob in [
        "docs/**",
        "docs/guide/*.png",
        "/README.md",
        "tests/test_*.py",
        "website/docs/**",
        "crates/*/tests/**",
        "{docs,tests}/**",
        "/docs/*.png",
        "docs/guide/**",
        "/docs/**",
        "docs/api/v2/**",
        "{docs,tests}/guide/**",
    ] {
        accepted(&periphery(glob));
    }
}

#[test]
fn a_periphery_folder_is_written_with_a_trailing_double_star() {
    for folder in [
        "docs/guide",
        "/docs",
        "tests",
        "docs/api/v2",
        "docs/",
        "/Makefile",
    ] {
        let message = problems(&periphery(folder));

        assert!(
            message.contains("review.areas[1].paths[0]")
                && message.contains("write the folder as `x/**`"),
            "`{folder}`: {message}"
        );
    }
    for folder in ["docs/guide", "tests", "docs"] {
        accepted(&periphery(&format!("{folder}/**")));
    }
}

#[test]
fn a_bare_file_name_is_read_at_every_depth_so_it_needs_a_leading_slash_to_stay_at_the_root() {
    let message = problems(&periphery("README.md"));

    assert!(message.contains(".github/review/**"), "{message}");
    accepted(&periphery("/README.md"));
}

#[test]
fn a_forced_name_of_the_root_is_not_forced_in_a_folder() {
    accepted(&periphery("tools/prek.toml"));
    accepted(&periphery("docs/botica.toml"));
}

#[test]
fn a_generated_glob_that_selects_some_protected_files_fails_however_narrow_it_is() {
    for glob in [
        "*.sql",
        "**/*.sql",
        "**/migrations/*.sql",
        "db/migrations/*.sql",
        "db/**/*.sql",
        "ci/*.yml",
        "ci/build.lock",
        ".github/**/*.yml",
        ".github/workflows/*.lock",
        "**/*.py",
        "*.lock",
        "uv.lock",
        "**/AGENTS.md",
        "**/{a,b}.md",
        "/db",
        "/.github",
        "/skills/",
        "/{db}",
        "/d*",
        "/.gith*",
        "/prek.t[]o]ml",
        "src/**/*.pb.go",
    ] {
        let message = problems(&generated(glob));

        assert!(
            message.contains("review.generated[0]"),
            "`{glob}`: {message}"
        );
    }
}

#[test]
fn a_generated_glob_that_selects_no_protected_file_loads() {
    for glob in [
        "/uv.lock",
        "/Cargo.lock",
        "schema/botica.schema.json",
        "src/gen/api.pb.go",
        "docs/api/openapi.json",
    ] {
        accepted(&generated(glob));
    }
}

#[test]
fn a_glob_with_many_brace_groups_is_refused_at_once_and_not_expanded() {
    let groups = "{a,b}".repeat(40);

    let started = Instant::now();
    let message = problems(&periphery(&format!("{groups}/x.txt")));

    assert!(message.contains("review.areas[1].paths[0]"), "{message}");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_glob_with_many_alternatives_in_one_group_is_refused() {
    let many = (0..40)
        .map(|n| format!("d{n}"))
        .collect::<Vec<_>>()
        .join(",");

    let message = problems(&periphery(&format!("{{{many}}}/x.txt")));

    assert!(message.contains("review.areas[1].paths[0]"), "{message}");
}
