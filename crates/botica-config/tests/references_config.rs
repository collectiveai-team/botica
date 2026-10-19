//! The `[references]` table of the configuration.

use botica_config::Config;

#[test]
fn the_table_lists_the_built_in_topics_that_the_project_replaces() {
    let config = Config::from_botica_toml("[references]\nreplace = [\"structured-logging\"]\n")
        .expect("parses");

    assert_eq!(config.references.replace, ["structured-logging"]);
}

#[test]
fn without_the_table_nothing_is_replaced() {
    let config = Config::from_botica_toml("").expect("parses");

    assert!(config.references.replace.is_empty());
    assert_eq!(config, Config::default());
}

#[test]
fn an_empty_table_replaces_nothing() {
    let config = Config::from_botica_toml("[references]\n").expect("parses");

    assert!(config.references.replace.is_empty());
}

#[test]
fn the_table_takes_no_other_key() {
    let message = Config::from_botica_toml("[references]\ncolour = \"red\"\n")
        .expect_err("rejected")
        .to_string();

    assert!(message.contains("colour"), "{message}");
}

#[test]
fn replace_is_a_list_of_ids() {
    assert!(Config::from_botica_toml("[references]\nreplace = \"structured-logging\"\n").is_err());
}

#[test]
fn the_table_works_in_the_tool_table_of_pyproject() {
    let config =
        Config::from_pyproject("[tool.botica.references]\nreplace = [\"in-memory-adapters\"]\n")
            .expect("parses")
            .expect("has a botica table");

    assert_eq!(config.references.replace, ["in-memory-adapters"]);
}
