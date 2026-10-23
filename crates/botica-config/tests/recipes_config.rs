//! The `[recipes]` table of the configuration.

use botica_config::Config;

#[test]
fn the_table_lists_the_built_in_recipes_that_the_project_replaces() {
    let config = Config::from_botica_toml("[recipes]\nreplace = [\"python\"]\n").expect("parses");

    assert_eq!(config.recipes.replace, ["python"]);
}

#[test]
fn without_the_table_nothing_is_replaced() {
    let config = Config::from_botica_toml("").expect("parses");

    assert!(config.recipes.replace.is_empty());
    assert_eq!(config, Config::default());
}

#[test]
fn an_empty_table_replaces_nothing() {
    let config = Config::from_botica_toml("[recipes]\n").expect("parses");

    assert!(config.recipes.replace.is_empty());
}

#[test]
fn the_table_takes_no_other_key() {
    let message = Config::from_botica_toml("[recipes]\ncolour = \"red\"\n")
        .expect_err("rejected")
        .to_string();

    assert!(message.contains("colour"), "{message}");
}

#[test]
fn replace_is_a_list_of_ids() {
    assert!(Config::from_botica_toml("[recipes]\nreplace = \"python\"\n").is_err());
}

#[test]
fn the_table_lists_the_recipes_that_the_project_installed() {
    let config = Config::from_botica_toml("[recipes]\ninstalled = [\"base-python\", \"team\"]\n")
        .expect("parses");

    assert_eq!(config.recipes.installed, ["base-python", "team"]);
    assert!(Config::from_botica_toml("")
        .expect("parses")
        .recipes
        .installed
        .is_empty());
}

#[test]
fn the_table_works_in_the_tool_table_of_pyproject() {
    let config = Config::from_pyproject("[tool.botica.recipes]\nreplace = [\"generic\"]\n")
        .expect("parses")
        .expect("has a botica table");

    assert_eq!(config.recipes.replace, ["generic"]);
}
