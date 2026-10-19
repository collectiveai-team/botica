//! `settings-env-schema` in `botica lint`: the run compares the schema at the root of the project
//! with the settings classes of the whole project, whatever files it is given.

mod common;

use common::{botica, project_in, stdout, write};

const SELECTS: &str = "preview = true\nextend-select = [\"settings-env-schema\"]\n";

const SETTINGS: &str = "from pydantic_settings import BaseSettings\n\n\
                        class Settings(BaseSettings):\n    database_url: str\n";

const SCHEMA: &str = ".env.schema";

fn project(name: &str, config: &str) -> std::path::PathBuf {
    let dir = project_in("settings-env-schema", name, Some(config));
    write(&dir, "src/app/settings.py", SETTINGS);
    write(&dir, "src/app/main.py", "x = 1\n");
    dir
}

#[test]
fn both_kinds_of_finding_come_out_whatever_file_botica_is_given() {
    let dir = project("both", SELECTS);
    write(&dir, SCHEMA, "API_KEY=\n");

    let output = botica(&dir, &["src/app/main.py"]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(
        text.contains(".env.schema:1:1: warning [settings-env-schema] CES-76"),
        "{text}"
    );
    assert!(
        text.contains("src/app/settings.py:4:5: warning [settings-env-schema] CES-76"),
        "{text}"
    );
}

#[test]
fn a_project_without_the_schema_or_without_the_rule_has_no_finding() {
    let dir = project("no-schema", SELECTS);
    assert!(!stdout(&botica(&dir, &[])).contains("settings-env-schema"));

    let dir = project("not-selected", "preview = true\n");
    write(&dir, SCHEMA, "API_KEY=\n");
    assert!(!stdout(&botica(&dir, &[])).contains("settings-env-schema"));
}

#[test]
fn the_rule_runs_with_the_project_rules_and_not_with_the_rules_of_one_file() {
    let dir = project("scope", SELECTS);
    write(&dir, SCHEMA, "API_KEY=\n");

    assert!(stdout(&botica(&dir, &["--scope", "project"])).contains("settings-env-schema"));
    assert!(!stdout(&botica(&dir, &["--scope", "file"])).contains("settings-env-schema"));
}

#[test]
fn the_paths_are_shown_from_where_botica_runs() {
    let dir = project("below", SELECTS);
    write(&dir, SCHEMA, "API_KEY=\n");

    let text = stdout(&botica(&dir.join("src/app"), &["main.py"]));

    let starts = |prefix: &str| text.lines().any(|line| line.starts_with(prefix));
    assert!(starts("../../.env.schema:1:1: warning"), "{text}");
    assert!(starts("settings.py:4:5: warning"), "{text}");
}

#[test]
fn a_file_that_the_configuration_excludes_holds_no_settings_class() {
    let dir = project(
        "excluded",
        &format!("{SELECTS}extend-exclude = [\"src/app/settings.py\"]\n"),
    );
    write(&dir, SCHEMA, "API_KEY=\n");

    let text = stdout(&botica(&dir, &[]));
    assert!(!text.contains("settings-env-schema"), "{text}");
}
