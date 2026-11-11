//! A glob of `periphery` or of `generated` that can select a path that a person has to review fails
//! to load, however narrow the glob is (ADR 0018 decision 1 and decision 6).

mod common;

use common::review::{accepted, problems, with_platform};

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
    ] {
        let text = periphery(glob);
        let parsed = botica_config::Config::from_botica_toml(&text).expect("parses");
        let error = parsed
            .validate(&common::review::catalog())
            .err()
            .unwrap_or_else(|| panic!("`{glob}` loads"));
        let message = error.to_string();
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
    ] {
        accepted(&periphery(glob));
    }
}

#[test]
fn a_bare_file_name_is_read_at_every_depth_so_it_needs_a_leading_slash_to_stay_at_the_root() {
    let message = problems(&periphery("README.md"));

    assert!(message.contains(".github/review/**"), "{message}");
    accepted(&periphery("/README.md"));
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
        "src/**/*.pb.go",
        "docs/api/openapi.json",
    ] {
        let config = format!(
            "[review]\ngenerated = [\"{glob}\"]\n\n{}",
            with_platform(
                "[[review.areas]]\nname = \"critical\"\npaths = [\"db/migrations/**\"]\nowners = [\"@o\"]\n"
            )
        );
        if glob == "src/**/*.pb.go" {
            let message = problems(&config);
            assert!(message.contains("**/.claude/**"), "{message}");
        } else {
            accepted(&config);
        }
    }
}

#[test]
fn a_glob_with_too_many_alternatives_is_read_as_selecting_everything() {
    let many = (0..70)
        .map(|n| format!("d{n}"))
        .collect::<Vec<_>>()
        .join(",");

    let message = problems(&periphery(&format!("{{{many}}}/x.txt")));

    assert!(message.contains("review.areas[1].paths[0]"), "{message}");
}
