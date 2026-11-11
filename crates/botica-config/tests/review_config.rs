//! The areas of the `review` table of the configuration (ADR 0018).

mod common;

use botica_config::{AreaName, ClassifierMode, Config, ReviewRole, Tier};
use common::review::{accepted, catalog, parse, problems, with_platform, PLATFORM};

#[test]
fn without_the_table_there_are_no_areas_and_the_classifier_has_its_starting_values() {
    let config = accepted("");

    assert!(config.review.areas.is_empty());
    assert_eq!(config.review.dir, None);
    assert!(config.review.generated.is_empty());
    assert_eq!(config.review.expected_roles(), ReviewRole::ALL);
    let classifier = &config.review.classifier;
    assert_eq!(classifier.mode, ClassifierMode::Shadow);
    assert!(classifier.required_checks.is_empty());
    assert_eq!(classifier.question_max, 0.1);
    assert_eq!(classifier.impact_min, 0.9);
    assert_eq!(classifier.min_confidence, 0.7);
    assert_eq!(classifier.max_changed_lines, 400);
}

#[test]
fn an_area_takes_the_floor_of_its_name_and_the_approvals_of_its_name_when_it_sets_neither() {
    let config = accepted(
        "[[review.areas]]\nname = \"platform\"\npaths = [\"ci/**\"]\nowners = [\"@org/platform\"]\n\n\
         [[review.areas]]\nname = \"core\"\npaths = [\"src/**\"]\n\n\
         [[review.areas]]\nname = \"periphery\"\npaths = [\"docs/**\"]\n",
    );

    let [platform, core, periphery] = config.review.areas.as_slice() else {
        panic!("three areas");
    };
    assert_eq!(platform.name, AreaName::Platform);
    assert_eq!(platform.effective_tier(), Tier::Medium);
    assert_eq!(platform.effective_approvals(), 1);
    assert_eq!(core.effective_tier(), Tier::Medium);
    assert_eq!(periphery.effective_tier(), Tier::Low);
    assert_eq!(periphery.effective_approvals(), 0);
}

#[test]
fn the_example_of_the_adr_parses_and_validates() {
    let config = accepted(&with_platform(
        "[[review.areas]]\nname = \"critical\"\npaths = [\"src/**/auth/**\", \"db/migrations/**\"]\n\
         owners = [\"@org/platform\"]\ntier = \"high\"\napprovals = 2\n",
    ));

    let critical = &config.review.areas[1];
    assert_eq!(critical.name, AreaName::Critical);
    assert_eq!(critical.paths, ["src/**/auth/**", "db/migrations/**"]);
    assert_eq!(critical.owners, ["@org/platform"]);
    assert_eq!(critical.effective_tier(), Tier::High);
    assert_eq!(critical.effective_approvals(), 2);
}

#[test]
fn the_table_is_read_from_the_tool_table_of_a_pyproject() {
    let text = "[project]\nname = \"x\"\n\n[tool.botica.review]\ngenerated = [\"/uv.lock\"]\n\n\
                [[tool.botica.review.areas]]\nname = \"platform\"\npaths = [\"ci/**\"]\nowners = [\"@me\"]\n";

    let config = Config::from_pyproject(text)
        .expect("parses")
        .expect("has the table");

    assert_eq!(config.review.generated, ["/uv.lock"]);
    assert_eq!(config.review.areas[0].owners, ["@me"]);
    config.validate(&catalog()).expect("valid");
}

#[test]
fn a_key_or_a_name_that_the_table_does_not_know_is_an_error_that_names_it() {
    for (text, marker) in [
        ("[review]\nowners = []\n", "unknown field `owners`"),
        (
            "[review.classifier]\nthreshold = 1\n",
            "unknown field `threshold`",
        ),
        (
            "[[review.areas]]\nname = \"core\"\npaths = [\"a\"]\nreviewers = []\n",
            "unknown field `reviewers`",
        ),
        (
            "[[review.areas]]\nname = \"important\"\npaths = [\"a\"]\n",
            "unknown variant `important`",
        ),
        ("[review]\nroles = [\"style\"]\n", "unknown variant `style`"),
        (
            "[review.classifier]\nmode = \"auto\"\n",
            "unknown variant `auto`",
        ),
        (
            "[[review.areas]]\npaths = [\"a\"]\n",
            "missing field `name`",
        ),
    ] {
        let message = problems(text);
        assert!(message.contains(marker), "{marker}: {message}");
    }
}

#[test]
fn a_tier_under_the_floor_of_the_name_is_an_error_and_a_tier_over_it_is_not() {
    for (name, tier) in [
        ("critical", "low"),
        ("critical", "medium"),
        ("platform", "low"),
        ("core", "low"),
    ] {
        let message = problems(&with_platform(&format!(
            "[[review.areas]]\nname = \"{name}\"\npaths = [\"a/**\"]\nowners = [\"@o\"]\ntier = \"{tier}\"\n"
        )));
        assert!(
            message.contains("review.areas[1].tier") && message.contains(name),
            "{name} at {tier}: {message}"
        );
    }
    for (name, tier) in [
        ("critical", "high"),
        ("core", "high"),
        ("periphery", "high"),
    ] {
        accepted(&with_platform(&format!(
            "[[review.areas]]\nname = \"{name}\"\npaths = [\"a/**\"]\nowners = [\"@o\"]\ntier = \"{tier}\"\n"
        )));
    }
}

#[test]
fn critical_asks_for_at_least_one_approval() {
    let message = problems(&with_platform(
        "[[review.areas]]\nname = \"critical\"\npaths = [\"a/**\"]\nowners = [\"@o\"]\napprovals = 0\n",
    ));

    assert!(message.contains("review.areas[1].approvals"), "{message}");
    accepted(&with_platform(
        "[[review.areas]]\nname = \"periphery\"\npaths = [\"a/**\"]\napprovals = 0\n",
    ));
    for name in ["platform", "core"] {
        let message = problems(&format!(
            "[[review.areas]]\nname = \"{name}\"\npaths = [\"a/**\"]\nowners = [\"@o\"]\napprovals = 0\n\n{}",
            if name == "platform" {
                "[[review.areas]]\nname = \"core\"\npaths = [\"b/**\"]\n"
            } else {
                PLATFORM
            }
        ));
        assert!(message.contains(".approvals"), "{name}: {message}");
    }
}

#[test]
fn critical_and_platform_need_owners_and_the_other_areas_do_not() {
    for name in ["critical", "platform"] {
        let message = problems(&format!(
            "[[review.areas]]\nname = \"{name}\"\npaths = [\"a/**\"]\n"
        ));
        assert!(
            message.contains("review.areas[0].owners") && message.contains(name),
            "{message}"
        );
    }
    accepted(&with_platform(
        "[[review.areas]]\nname = \"core\"\npaths = [\"a/**\"]\n\n\
         [[review.areas]]\nname = \"periphery\"\npaths = [\"b/**\"]\n",
    ));
}

#[test]
fn an_owner_is_a_user_a_team_or_an_address_with_no_blanks() {
    accepted(&with_platform(
        "[[review.areas]]\nname = \"core\"\npaths = [\"a/**\"]\n\
         owners = [\"@user\", \"@org/team\", \"@org/group/team\", \"dev@example.com\", \"a_b+c@x-y.org\"]\n",
    ));
    for bad in [
        "", "user", "@", "@org/", "@/team", "@a b", "@a\\n@b", "@a;rm", "#x@y", "@a/b/c/d", "a@",
        "@@a", "a@b",
    ] {
        let message = problems(&with_platform(&format!(
            "[[review.areas]]\nname = \"core\"\npaths = [\"a/**\"]\nowners = [\"{bad}\"]\n"
        )));
        assert!(
            message.contains("review.areas[1].owners[0]"),
            "`{bad}`: {message}"
        );
    }
}

#[test]
fn the_paths_of_an_entry_are_required_and_are_globs_that_select_something() {
    assert!(problems(&with_platform(
        "[[review.areas]]\nname = \"core\"\npaths = []\n"
    ))
    .contains("review.areas[1].paths"));
    for bad in ["", "   ", "!a/**", "# a", "a\\nb", " **/*", "a/** ", "  *"] {
        let message = problems(&with_platform(&format!(
            "[[review.areas]]\nname = \"core\"\npaths = [\"{bad}\"]\n"
        )));
        assert!(
            message.contains("review.areas[1].paths[0]"),
            "`{bad}`: {message}"
        );
    }
    assert!(problems(&with_platform(
        "[[review.areas]]\nname = \"core\"\npaths = [\"{a\"]\n"
    ))
    .contains("invalid glob"));
}

#[test]
fn a_name_may_come_in_several_entries_but_a_glob_may_not() {
    accepted(&with_platform(
        "[[review.areas]]\nname = \"core\"\npaths = [\"a/**\"]\n\n\
         [[review.areas]]\nname = \"core\"\npaths = [\"b/**\"]\nowners = [\"@o\"]\n",
    ));

    let across = problems(&with_platform(
        "[[review.areas]]\nname = \"core\"\npaths = [\"a/**\"]\n\n\
         [[review.areas]]\nname = \"periphery\"\npaths = [\"b/**\", \"a/**\"]\n",
    ));
    assert!(
        across.contains("review.areas[2].paths[1]")
            && across.contains("`a/**`")
            && across.contains("review.areas[1].paths[0]"),
        "{across}"
    );
    let within = problems(&with_platform(
        "[[review.areas]]\nname = \"core\"\npaths = [\"a/**\", \"a/**\"]\n",
    ));
    assert!(within.contains("review.areas[1].paths[1]"), "{within}");
}

#[test]
fn a_table_with_areas_needs_a_platform_entry() {
    let message = problems("[[review.areas]]\nname = \"core\"\npaths = [\"a/**\"]\n");

    assert!(
        message.contains("review.areas") && message.contains("platform"),
        "{message}"
    );
    accepted("[review]\ngenerated = []\n");
}

#[test]
fn a_periphery_entry_cannot_cover_the_paths_that_are_platform_whatever_the_entries_say() {
    for glob in [
        "**",
        "**/*.md",
        "*.toml",
        ".github/**",
        ".github/workflows/**",
        "prek.toml",
        ".abide/**",
        "**/AGENTS.md",
        "**/SKILL.md",
        "botica.toml",
        ".claude/**",
    ] {
        let message = problems(&with_platform(&format!(
            "[[review.areas]]\nname = \"periphery\"\npaths = [\"{glob}\"]\n"
        )));
        assert!(
            message.contains("review.areas[1].paths[0]") && message.contains("platform"),
            "`{glob}`: {message}"
        );
    }
    accepted(&with_platform(
        "[[review.areas]]\nname = \"periphery\"\npaths = [\"docs/**\", \"website/docs/**\", \"/README.md\", \"tests/**\"]\n",
    ));
}

#[test]
fn the_folder_of_the_roles_is_platform_too() {
    let config = "[review]\ndir = \"policy/review\"\n\n";

    let message = problems(&format!(
        "{config}{}",
        with_platform("[[review.areas]]\nname = \"periphery\"\npaths = [\"policy/**\"]\n")
    ));
    assert!(message.contains("policy/review"), "{message}");

    let loaded = parse(config);
    assert!(loaded
        .review
        .forced_platform_globs()
        .contains(&"policy/review/**".to_owned()));
    assert!(!parse("")
        .review
        .forced_platform_globs()
        .iter()
        .any(|glob| glob.contains("policy")));
}

#[test]
fn the_forced_platform_paths_are_the_ones_of_the_adr_and_pyproject_toml() {
    let mut globs = parse("").review.forced_platform_globs();
    let mut expected = vec![
        "/botica.toml",
        "/pyproject.toml",
        ".github/review/**",
        ".gitlab/review/**",
        "/skills-lock.json",
        ".github/workflows/**",
        ".github/actions/**",
        "/.gitlab-ci.yml",
        ".github/CODEOWNERS",
        ".gitlab/CODEOWNERS",
        "/.gitattributes",
        "/prek.toml",
        ".abide/**",
        "**/AGENTS.md",
        "**/CLAUDE.md",
        "**/.claude/**",
        "**/.mcp.json",
        "**/.cursorrules",
        "**/INVARIANTS.md",
        "**/REVIEW.md",
        "**/SKILL.md",
    ];

    globs.sort();
    expected.sort_unstable();

    assert_eq!(globs, expected);
}
