//! A configuration that names a retired name or a removed rule (ADR 0016, decisions 2 and 3).

use botica_catalog::Catalog;
use botica_config::{CommandLine, Config, Resolution, Use};

fn catalog() -> Catalog {
    Catalog::embedded().expect("the built-in catalog loads")
}

fn config(text: &str) -> Config {
    Config::from_botica_toml(text).unwrap_or_else(|error| panic!("should parse:\n{error}"))
}

fn warnings(text: &str, turn_on: Use) -> Vec<String> {
    config(text)
        .validate(&catalog(), turn_on)
        .unwrap_or_else(|error| panic!("should load:\n{error}"))
}

fn error(text: &str) -> String {
    config(text)
        .validate(&catalog(), Use::TurnsOn)
        .expect_err("should be rejected")
        .to_string()
}

const REMOVED: &str =
    "rule `no-console-log` was removed in 0.10.0 and `log-no-print` replaces it, so it no longer runs";

#[test]
fn a_retired_code_selects_its_rule_with_a_warning() {
    let text = "preview = true\nselect = [\"BTARCH005\"]\n";

    assert_eq!(
        warnings(text, Use::TurnsOn),
        ["`BTARCH005` is the old code of `db-no-create-all` (BTDB002 since 0.13.0)"]
    );
    let resolution = Resolution::new(&config(text), &catalog()).expect("it resolves");
    let active: Vec<&str> = resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
        .map(|entry| entry.rule.id.as_str())
        .collect();
    assert_eq!(active, ["db-no-create-all"]);
    assert_eq!(resolution.warnings().len(), 1);
}

#[test]
fn each_retired_name_warns_once_wherever_the_configuration_names_it() {
    let text = "ignore = [\"arch-database-package\"]\n\
                [rules.arch-database-package]\nseverity = \"error\"\n\
                [[overrides]]\nfiles = [\"a/**\"]\nignore = [\"arch-database-package\"]\n";

    assert_eq!(
        warnings(text, Use::TurnsOn),
        ["`arch-database-package` is the old id of `database-package` (database-package since 0.13.0)"]
    );
}

#[test]
fn a_removed_rule_that_the_configuration_turns_on_stops_the_run() {
    for text in [
        "select = [\"no-console-log\"]\n",
        "extend-select = [\"BTLOG003\"]\n",
        "[rules.no-console-log]\nenabled = true\n",
    ] {
        assert!(error(text).contains(REMOVED), "{text}: {}", error(text));
    }
}

#[test]
fn a_removed_rule_anywhere_else_is_a_warning() {
    for text in [
        "ignore = [\"no-console-log\"]\n",
        "[rules.no-console-log]\nseverity = \"error\"\n",
        "[[overrides]]\nfiles = [\"a/**\"]\nignore = [\"BTLOG003\"]\n",
    ] {
        assert_eq!(warnings(text, Use::TurnsOn), [REMOVED], "{text}");
    }
}

#[test]
fn a_command_that_describes_the_configuration_goes_on_past_a_removed_rule_it_turns_on() {
    let text = "select = [\"no-console-log\"]\n";

    assert_eq!(warnings(text, Use::TurnsOnDescribed), [REMOVED]);
    assert!(Resolution::describing(&config(text), &catalog()).is_ok());
}

#[test]
fn the_command_line_stops_on_a_removed_rule_it_selects_and_not_on_one_it_ignores() {
    let selects = CommandLine {
        select: vec!["no-console-log".to_owned()],
        ..CommandLine::default()
    };
    let ignores = CommandLine {
        ignore: vec!["no-console-log".to_owned()],
        ..CommandLine::default()
    };

    let error = Config::default()
        .with_command_line(&catalog(), &selects, Use::TurnsOn)
        .expect_err("--select of a removed rule stops")
        .to_string();
    assert!(error.contains("--select"), "{error}");
    assert!(Config::default()
        .with_command_line(&catalog(), &selects, Use::TurnsOnDescribed)
        .is_ok());
    assert!(Config::default()
        .with_command_line(&catalog(), &ignores, Use::TurnsOn)
        .is_ok());
}

#[test]
fn a_name_that_no_release_published_is_an_error_everywhere() {
    for text in [
        "select = [\"no-such-rule\"]\n",
        "ignore = [\"BTARCH099\"]\n",
    ] {
        assert!(error(text).contains("unknown rule"), "{text}");
    }
}
