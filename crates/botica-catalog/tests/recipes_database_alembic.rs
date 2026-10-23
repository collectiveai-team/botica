//! The recipe `database-alembic`: the configuration of the command `alembic` and the hook of prek
//! that runs `alembic check` (CES-18).

use std::collections::BTreeMap;

use botica_catalog::{render, Recipes};

#[test]
fn database_alembic_adds_the_configuration_of_alembic_and_a_hook_that_runs_alembic_check() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let recipe = recipes
        .recipe("database-alembic")
        .expect("database-alembic");

    assert_eq!(recipe.extends, ["base-python"]);
    assert_eq!(recipe.rules, ["db-no-create-all"]);
    assert!(recipe.tools.is_empty() && recipe.files.is_empty());
    let sections: Vec<String> = recipe
        .sections
        .iter()
        .map(|s| format!("{} {}", s.path, s.marker))
        .collect();
    assert_eq!(
        sections,
        [
            "pyproject.toml # MARK: Alembic",
            "prek.toml # MARK: alembic-check"
        ]
    );

    let mut resolved = recipes.resolve("database-alembic").expect("resolves");
    recipes
        .set_params(
            &mut resolved,
            &BTreeMap::from([(
                "database-url-variable".to_owned(),
                "ORDERS_DATABASE_URL".to_owned(),
            )]),
            &BTreeMap::new(),
        )
        .expect("sets");
    let section = |path: &str| {
        let text = recipes
            .template("database-alembic", path)
            .expect("a template");
        render(text, &resolved.params).expect("renders")
    };

    let pyproject = section("sections/pyproject-alembic.toml");
    assert!(
        pyproject.contains("script_location = \"app.database:migrations\"\n"),
        "{pyproject}"
    );
    let hook = section("sections/prek-alembic.toml");
    assert!(hook.contains("export ORDERS_DATABASE_URL="), "{hook}");
    assert!(
        hook.contains("uv run --frozen alembic upgrade head && uv run --frozen alembic check"),
        "{hook}"
    );
}
