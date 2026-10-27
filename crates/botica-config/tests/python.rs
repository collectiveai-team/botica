use botica_catalog::{Catalog, SourceFile};
use botica_config::{Config, ProjectPython, Reason, Resolution};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

fn rule_toml(slug: &str, code: &str, python: Option<&str>) -> String {
    let python = python.map_or_else(String::new, |range| format!("python = \"{range}\"\n"));
    format!(
        "id = \"{slug}\"\ncode = \"{code}\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         scope = \"file\"\nneeds = \"path\"\nrun-on = [\"cli\"]\ndefault-enabled = true\n\
         default-severity = \"error\"\n{python}[[implementations]]\nkind = \"native\"\n\
         languages = [\"python\"]\n"
    )
}

fn catalog(rules: &[(&str, &str, Option<&str>)]) -> Result<Catalog, String> {
    let mut files = vec![SourceFile {
        path: "structure/family.toml".into(),
        text: "code = \"STRUC\"\ndescription = \"d\"\n".into(),
    }];
    for (slug, code, python) in rules {
        files.push(SourceFile {
            path: format!("structure/{slug}/rule.toml"),
            text: rule_toml(slug, code, *python),
        });
        files.push(SourceFile {
            path: format!("structure/{slug}/README.md"),
            text: README.into(),
        });
    }
    Catalog::from_files(&files).map_err(|error| error.to_string())
}

fn gated() -> Catalog {
    catalog(&[
        ("any-python", "BTSTRUC001", None),
        ("before-315", "BTSTRUC002", Some("<3.15")),
        ("from-315", "BTSTRUC003", Some(">=3.15")),
    ])
    .unwrap_or_else(|error| panic!("catalog:\n{error}"))
}

fn resolve(requires: Option<&str>) -> Resolution {
    let python = ProjectPython::from_requires(requires.map(str::to_owned));
    Resolution::new(&Config::default(), &gated())
        .expect("valid config")
        .only_python(&python)
}

fn active(resolution: &Resolution) -> Vec<&str> {
    let mut slugs: Vec<&str> = resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
        .map(|entry| entry.rule.id.as_str())
        .collect();
    slugs.sort_unstable();
    slugs
}

#[test]
fn the_lower_bound_of_requires_python_picks_the_rules_whose_range_holds_it() {
    assert_eq!(
        active(&resolve(Some(">=3.12, <4"))),
        ["any-python", "before-315"]
    );
    assert_eq!(active(&resolve(Some(">=3.15"))), ["any-python", "from-315"]);
    assert_eq!(
        active(&resolve(Some("~=3.16.0"))),
        ["any-python", "from-315"]
    );
}

#[test]
fn a_project_without_a_lower_bound_counts_as_the_default_of_ces_77() {
    assert_eq!(active(&resolve(None)), ["any-python", "before-315"]);
    assert_eq!(active(&resolve(Some("<4"))), ["any-python", "before-315"]);
}

#[test]
fn a_rule_left_out_by_its_range_says_what_each_side_asks() {
    let resolution = resolve(Some(">=3.12"));
    assert_eq!(
        resolution.entry("from-315").expect("rule").reason,
        Reason::OutsidePython {
            range: ">=3.15".to_owned(),
            project: "the project requires `>=3.12`".to_owned(),
        }
    );
    assert_eq!(
        resolution
            .entry("from-315")
            .expect("rule")
            .reason
            .to_string(),
        "requires Python `>=3.15`, and the project requires `>=3.12`"
    );
    assert_eq!(
        resolve(None)
            .entry("from-315")
            .expect("rule")
            .reason
            .to_string(),
        "requires Python `>=3.15`, and the project names no `requires-python`, so botica takes \
         3.14 (CES-77)"
    );
}

#[test]
fn a_rule_that_is_not_selected_keeps_its_own_reason() {
    let config: Config = toml::from_str("ignore = [\"from-315\"]").expect("config");
    let resolution = Resolution::new(&config, &gated())
        .expect("valid config")
        .only_python(&ProjectPython::from_requires(Some(">=3.12".to_owned())));
    assert_eq!(
        resolution.entry("from-315").expect("rule").reason,
        Reason::IgnoredBy("from-315".to_owned())
    );
}

#[test]
fn a_range_that_is_not_a_list_of_clauses_does_not_load() {
    let error = catalog(&[("odd", "BTSTRUC001", Some("3.15+"))]).expect_err("it is refused");
    assert!(
        error.contains("`python` is a range of Python versions"),
        "{error}"
    );
}
