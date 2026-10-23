//! `recipe-conflict`: where the finding about two recipes that ask for one package with different
//! requirements sits, and what it says.

use botica_catalog::{RecipeItem, ToolConflict};
use botica_core::recipe_conflict::{check, unresolved, RecipeFile};
use botica_core::Severity;

const TEAM: &str =
    "description = \"The Python projects of the team\"\nextends = [\"base-python\"]\n\n\
                    [[tools]]\nmanager = \"uv\"\npackages = [\"ruff>=0.15\", \"pytest-cov\"]\n";

fn conflict(package: &str, first: (&str, &str), then: (&str, &str)) -> ToolConflict {
    let item = |(requirement, recipe): (&str, &str)| RecipeItem {
        id: requirement.to_owned(),
        from: recipe.to_owned(),
    };
    ToolConflict {
        manager: "uv".to_owned(),
        group: "dev".to_owned(),
        package: package.to_owned(),
        first: item(first),
        then: item(then),
    }
}

fn team() -> RecipeFile<'static> {
    RecipeFile {
        id: "team",
        path: "recipes/team.toml",
        text: TEAM,
    }
}

#[test]
fn the_finding_sits_on_the_requirement_of_the_later_recipe_and_names_both() {
    let found = check(
        &[conflict(
            "ruff",
            ("ruff", "base-python"),
            ("ruff>=0.15", "team"),
        )],
        &[team()],
        "botica.toml",
        Severity::Error,
    );

    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].path.as_str(), found[0].line, found[0].column),
        ("recipes/team.toml", 6, 13)
    );
    assert_eq!(found[0].end - found[0].start, "\"ruff>=0.15\"".len());
    assert_eq!(
        found[0].message,
        "The recipes `team` and `base-python` ask for the package `ruff` as `ruff>=0.15` and as \
         `ruff`: a package has one requirement across the recipes, and `botica recipe install` \
         stops on them"
    );
}

#[test]
fn when_the_later_recipe_is_built_in_the_finding_sits_on_the_earlier_one() {
    let found = check(
        &[conflict(
            "ruff",
            ("ruff>=0.15", "team"),
            ("ruff", "base-python"),
        )],
        &[team()],
        "botica.toml",
        Severity::Error,
    );

    assert_eq!(
        (found[0].path.as_str(), found[0].line),
        ("recipes/team.toml", 6)
    );
}

#[test]
fn when_both_are_built_in_the_finding_is_about_the_configuration() {
    let found = check(
        &[conflict(
            "ruff",
            ("ruff", "base-python"),
            ("ruff>=0.15", "lint-strict"),
        )],
        &[team()],
        "botica.toml",
        Severity::Error,
    );

    assert_eq!(found[0].path, "botica.toml");
    assert!(found[0].file_level);
}

#[test]
fn a_project_whose_recipes_do_not_resolve_has_a_finding_about_the_file() {
    let found = unresolved(
        "botica.toml",
        "no recipe `nope` (`botica recipe list` lists them)",
        Severity::Error,
    );

    assert!(found.file_level);
    assert_eq!(
        found.message,
        "botica cannot resolve the recipes of the project, so it does not look for conflicts \
         between them: no recipe `nope` (`botica recipe list` lists them)"
    );
}
