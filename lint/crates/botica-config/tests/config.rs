use botica_catalog::{Catalog, SourceFile};
use botica_config::{Config, RuleSeverity};

fn catalog() -> Catalog {
    Catalog::embedded().expect("the built-in catalog loads")
}

fn parse(text: &str) -> Config {
    Config::from_botica_toml(text).unwrap_or_else(|error| panic!("should parse:\n{error}"))
}

fn problems_in(text: &str, catalog: &Catalog) -> String {
    match Config::from_botica_toml(text) {
        Err(error) => error.to_string(),
        Ok(config) => config
            .validate(catalog)
            .expect_err("the config should be rejected")
            .to_string(),
    }
}

fn problems(text: &str) -> String {
    problems_in(text, &catalog())
}

fn accepted(text: &str) {
    parse(text)
        .validate(&catalog())
        .unwrap_or_else(|error| panic!("should be valid:\n{error}"));
}

#[test]
fn an_empty_file_gives_the_house_defaults() {
    let config = parse("");
    assert_eq!(config, Config::default());
    assert_eq!(config.select, ["default"]);
    assert_eq!(config.rule_dirs, [".botica/rules"]);
    assert!(!config.preview);
    accepted("");
}

#[test]
fn the_example_of_the_spec_parses_and_validates() {
    accepted(
        r#"
select = ["default"]
extend-select = ["BTSTRUC", "tag:size"]
ignore = ["no-utils"]
preview = false
languages = ["python", "typescript"]
extend-exclude = ["vendor/**"]
rule-dirs = [".botica/rules"]
external = ["acme-legacy/*"]

[catalogs]
shared = { path = "../shared-rules" }

[rules.file-size-guard]
severity = "error"
warn = 400
max = 700
extend-ignores = ["docs/**/*.rst"]

[[overrides]]
files = ["tests/**"]
ignore = ["file-size-guard"]
severity = { no-utils = "warning" }
"#,
    );
    let config = parse("[rules.file-size-guard]\nseverity = \"off\"\nwarn = 10\n");
    let rule = &config.rules["file-size-guard"];
    assert_eq!(rule.severity, Some(RuleSeverity::Off));
    assert_eq!(rule.options["warn"].as_integer(), Some(10));
}

#[test]
fn an_unknown_key_is_an_error_that_names_it() {
    assert!(problems("slect = [\"all\"]").contains("slect"));
    assert!(problems("[[overrides]]\nfiles = [\"a\"]\nignor = []\n").contains("ignor"));
    assert!(problems("[catalogs]\nacme = { git = \"https://example.com/x\" }\n").contains("git"));
}

#[test]
fn a_rule_option_that_the_rule_does_not_declare_lists_the_valid_ones() {
    let message = problems("[rules.file-size-guard]\nwrn = 10\n");
    assert!(
        message.contains("wrn") && message.contains("warn") && message.contains("max"),
        "{message}"
    );
}

#[test]
fn rule_options_are_checked_against_their_type_and_minimum() {
    assert!(problems("[rules.file-size-guard]\nwarn = \"many\"\n").contains("integer"));
    assert!(problems("[rules.file-size-guard]\nwarn = 0\n").contains("at least 1"));
    accepted("[rules.file-size-guard]\nwarn = 1\nmax = 2\n");
}

#[test]
fn an_unknown_rule_id_is_an_error_wherever_it_appears() {
    for text in [
        "ignore = [\"no-such-rule\"]",
        "select = [\"no-such-rule\"]",
        "extend-select = [\"BTSTRUC999\"]",
        "[rules.no-such-rule]\nseverity = \"off\"\n",
        "[replace]\nno-such-rule = \"no-utils\"\n",
        "[replace]\nno-utils = \"no-such-rule\"\n",
        "[[overrides]]\nfiles = [\"a\"]\nseverity = { no-such-rule = \"info\" }\n",
        "[[overrides]]\nfiles = [\"a\"]\nignore = [\"no-such-rule\"]\n",
    ] {
        assert!(
            problems(text).contains("no-such-rule") || problems(text).contains("BTSTRUC999"),
            "{text}"
        );
    }
}

#[test]
fn rules_can_be_named_by_slug_code_or_alias_but_only_once() {
    accepted("[rules.BTSTRUC001]\nseverity = \"warning\"\n");
    let message = problems(
        "[rules.file-size-guard]\nseverity = \"info\"\n[rules.BTSTRUC001]\nseverity = \"off\"\n",
    );
    assert!(
        message.contains("twice") || message.contains("same rule"),
        "{message}"
    );
}

#[test]
fn selectors_follow_the_family_rules_of_the_spec() {
    for ok in [
        "default",
        "all",
        "BTSTRUC",
        "BTSTRUC0",
        "BTSTRUC00",
        "BTSTRUC001",
        "BT001",
        "BT0",
        "no-utils",
        "tag:size",
    ] {
        accepted(&format!("select = [\"{ok}\"]"));
    }
    for bad in [
        "BT",
        "BTNOPE",
        "BTSTRUC9",
        "BTSTRUC0001",
        "CES-71",
        "Tag:size",
        "tag:",
        "tag:no-such-tag",
        "NoUtils",
        "",
    ] {
        let message = problems(&format!("select = [\"{bad}\"]"));
        assert!(message.contains("select[0]"), "{bad}: {message}");
    }
}

#[test]
fn an_empty_family_can_be_selected_but_a_prefix_must_match_a_rule() {
    accepted("select = [\"BTTEST\"]");
    assert!(problems("select = [\"BTTEST0\"]").contains("select[0]"));
}

#[test]
fn a_removed_rule_cannot_be_mentioned() {
    let removed = "id = \"old-rule\"\ncode = \"BTSTRUC009\"\nsummary = \"s\"\nstatus = \"removed\"\nsince = \"0.1.0\"\n\
                   scope = \"file\"\nneeds = \"path\"\nrun-on = [\"cli\"]\ndefault-enabled = false\ndefault-severity = \"error\"\n\
                   [[implementations]]\nkind = \"native\"\nlanguages = [\"*\"]\n";
    let files = vec![
        SourceFile {
            path: "structure/family.toml".into(),
            text: "code = \"STRUC\"\ndescription = \"d\"\n".into(),
        },
        SourceFile {
            path: "structure/old-rule/rule.toml".into(),
            text: removed.into(),
        },
        SourceFile {
            path: "structure/old-rule/README.md".into(),
            text: "# r\n".into(),
        },
    ];
    let catalog = Catalog::from_files(&files).expect("catalog with a removed rule");
    let message = problems_in("ignore = [\"old-rule\"]", &catalog);
    assert!(message.contains("removed"), "{message}");
}

#[test]
fn globs_must_be_valid() {
    assert!(problems("extend-exclude = [\"[unclosed\"]").contains("extend-exclude[0]"));
    assert!(problems("external = [\"{a\"]").contains("external[0]"));
    assert!(
        problems("[rules.file-size-guard]\nextend-ignores = [\"[x\"]\n").contains("extend-ignores")
    );
    assert!(problems("[[overrides]]\nfiles = [\"[x\"]\n").contains("overrides[0].files[0]"));
    assert!(problems("[[overrides]]\nfiles = []\n").contains("files"));
}

#[test]
fn every_problem_is_reported_at_once() {
    let message =
        problems("select = [\"nope\"]\nignore = [\"nada\"]\n[rules.file-size-guard]\nwrn = 1\n");
    assert!(
        message.contains("nope") && message.contains("nada") && message.contains("wrn"),
        "{message}"
    );
}

#[test]
fn pyproject_uses_the_tool_botica_table_and_ignores_the_rest() {
    let text = "[project]\nname = \"x\"\n\n[tool.ruff]\nline-length = 100\n\n[tool.botica]\nselect = [\"BTSTRUC\"]\n";
    let config = Config::from_pyproject(text)
        .expect("parses")
        .expect("has a botica table");
    assert_eq!(config.select, ["BTSTRUC"]);

    assert!(Config::from_pyproject("[project]\nname = \"x\"\n")
        .expect("parses")
        .is_none());
    assert!(Config::from_pyproject("[tool.ruff]\nline-length = 100\n")
        .expect("parses")
        .is_none());

    let error = Config::from_pyproject("[tool.botica]\nslect = []\n").expect_err("unknown key");
    assert!(error.to_string().contains("slect"), "{error}");
    assert!(Config::from_pyproject("[tool.botica\n").is_err());
}
