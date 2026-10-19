//! An external rule: a rule of the catalog that another tool enforces through its configuration.

mod common;

use botica_catalog::{Kind, SourceFile};
use common::{catalog, family, file, issues};

/// The root of the `rule.toml` of `cognitive-complexity`, an external rule of `structure`.
const HEAD: &str = "id = \"cognitive-complexity\"\ncode = \"BTSTRUC001\"\nstandard = \"CES-110\"\n\
summary = \"Functions stay under a cognitive complexity of 15\"\nstatus = \"preview\"\n\
default-enabled = false\ndefault-severity = \"error\"\n";

/// The fields of an external implementation of complexipy.
const COMPLEXIPY: &str = "[[implementations]]\nkind = \"external\"\nlanguages = [\"python\"]\n\
tool = \"complexipy\"\ndocs = \"https://rohaquinlop.github.io/complexipy/\"\n";

const FRAGMENT: &str = "[[repos]]\n[[repos.hooks]]\nid = \"complexipy\"\n";

/// A README with the sections of an external rule.
const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

fn external(head_extra: &str, implementations: &str, extra: Vec<SourceFile>) -> Vec<SourceFile> {
    let mut files = vec![
        family("structure", "STRUC"),
        file(
            "structure/cognitive-complexity/rule.toml",
            &format!("{HEAD}{head_extra}\n{implementations}"),
        ),
        file("structure/cognitive-complexity/README.md", README),
    ];
    files.extend(extra);
    files
}

fn fragment(name: &str, text: &str) -> SourceFile {
    file(&format!("structure/cognitive-complexity/{name}"), text)
}

#[test]
fn an_external_rule_loads_with_its_tool_and_the_fragment_of_its_configuration() {
    let implementation = format!("{COMPLEXIPY}codes = [\"C901\"]\nconfig = \"config/prek.toml\"\n");
    let loaded = catalog(vec![external(
        "",
        &implementation,
        vec![fragment("config/prek.toml", FRAGMENT)],
    )])
    .unwrap_or_else(|error| panic!("{error}"));

    let rule = loaded.resolve("cognitive-complexity").expect("the rule");
    assert!(rule.is_external_only());
    assert_eq!(rule.scope, None);
    let implementation = &rule.implementations[0];
    assert_eq!(implementation.kind, Kind::External);
    assert_eq!(implementation.tool.as_deref(), Some("complexipy"));
    assert_eq!(
        implementation.docs.as_deref(),
        Some("https://rohaquinlop.github.io/complexipy/")
    );
    assert_eq!(implementation.codes, ["C901"]);
    assert_eq!(implementation.config.as_deref(), Some("config/prek.toml"));
    assert_eq!(implementation.target(), Some("prek.toml"));
    assert_eq!(implementation.source.as_deref(), Some(FRAGMENT));
}

#[test]
fn an_external_implementation_may_have_no_codes_and_no_configuration() {
    let loaded =
        catalog(vec![external("", COMPLEXIPY, vec![])]).unwrap_or_else(|error| panic!("{error}"));

    let implementation = &loaded
        .resolve("cognitive-complexity")
        .expect("the rule")
        .implementations[0];
    assert!(implementation.codes.is_empty());
    assert_eq!(implementation.target(), None);
    assert_eq!(implementation.source, None);
}

#[test]
fn an_external_implementation_needs_its_tool_and_its_docs() {
    for field in ["tool", "docs"] {
        let implementation: String = COMPLEXIPY
            .lines()
            .filter(|line| !line.starts_with(field))
            .map(|line| format!("{line}\n"))
            .collect();

        let message = issues(vec![external("", &implementation, vec![])]);

        assert!(
            message.contains(&format!("an external implementation needs `{field}`")),
            "{field}: {message}"
        );
    }
}

#[test]
fn a_rule_with_external_implementations_only_has_no_scope_needs_run_on_or_prefilter() {
    for (field, line) in [
        ("scope", "scope = \"file\""),
        ("needs", "needs = \"syntax\""),
        ("run-on", "run-on = [\"cli\"]"),
        ("prefilter", "prefilter = \"x\""),
    ] {
        let message = issues(vec![external(line, COMPLEXIPY, vec![])]);

        assert!(
            message.contains(&format!(
                "`{field}` applies to a rule that `botica lint` runs, and another tool enforces \
                 this one"
            )),
            "{field}: {message}"
        );
    }
}

#[test]
fn a_rule_without_implementations_is_not_a_rule_that_another_tool_enforces() {
    let message = issues(vec![external("scope = \"file\"", "", vec![])]);

    assert!(
        message.contains("a rule needs at least one implementation"),
        "{message}"
    );
    assert!(!message.contains("another tool enforces"), "{message}");
}

#[test]
fn an_external_implementation_beside_a_deterministic_one_needs_scope_needs_and_run_on() {
    let mixed = format!(
        "{COMPLEXIPY}\n[[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"typescript\"]\n\
         file = \"typescript.yml\"\n"
    );
    let yaml = fragment(
        "typescript.yml",
        "id: cognitive-complexity\nlanguage: typescript\nrule:\n  pattern: x\n",
    );

    let message = issues(vec![external("", &mixed, vec![yaml.clone()])]);
    assert!(
        message.contains("a deterministic rule needs `scope`"),
        "{message}"
    );

    let deterministic = "scope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]";
    let loaded = catalog(vec![external(deterministic, &mixed, vec![yaml])])
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(!loaded
        .resolve("cognitive-complexity")
        .expect("the rule")
        .is_external_only());
}

#[test]
fn the_fields_of_an_external_tool_belong_to_an_external_implementation_only() {
    for (field, line) in [
        ("tool", "tool = \"ruff\""),
        ("docs", "docs = \"https://docs.astral.sh/ruff/\""),
        ("codes", "codes = [\"C901\"]"),
        ("config", "config = \"config/pyproject.toml\""),
    ] {
        let native = format!(
            "scope = \"file\"\nneeds = \"path\"\nrun-on = [\"cli\"]\n\n[[implementations]]\n\
             kind = \"native\"\nlanguages = [\"python\"]\n{line}\n"
        );
        let (head_extra, implementation) = native.split_once("\n\n").expect("two parts");

        let message = issues(vec![external(head_extra, implementation, vec![])]);

        assert!(
            message.contains(&format!(
                "implementation 0: `{field}` applies to an external implementation only"
            )),
            "{field}: {message}"
        );
    }
}

#[test]
fn the_configuration_is_a_toml_file_of_the_folder_config() {
    for config in [
        "prek.toml",
        "config/prek.yaml",
        "config/a/prek.toml",
        "config/.toml",
        "../prek.toml",
    ] {
        let implementation = format!("{COMPLEXIPY}config = \"{config}\"\n");

        let message = issues(vec![external("", &implementation, vec![])]);

        assert!(
            message.contains(&format!(
                "config `{config}` must be `config/<file>.toml`, the file of the project it goes into"
            )),
            "{config}: {message}"
        );
    }
}

#[test]
fn a_fragment_that_is_missing_is_not_toml_or_that_no_implementation_names_is_rejected() {
    let named = format!("{COMPLEXIPY}config = \"config/prek.toml\"\n");

    let missing = issues(vec![external("", &named, vec![])]);
    assert!(
        missing.contains("`config/prek.toml` is named by an implementation of `cognitive-complexity` and is missing"),
        "{missing}"
    );

    let broken = issues(vec![external(
        "",
        &named,
        vec![fragment("config/prek.toml", "[[repos]\n")],
    )]);
    assert!(
        broken.contains("structure/cognitive-complexity/config/prek.toml"),
        "{broken}"
    );

    let unnamed = issues(vec![external(
        "",
        COMPLEXIPY,
        vec![fragment("config/prek.toml", FRAGMENT)],
    )]);
    assert!(
        unnamed.contains("no implementation names this file with `config`"),
        "{unnamed}"
    );
}

#[test]
fn the_readme_of_an_external_rule_names_its_tool_and_its_configuration() {
    for missing in ["Tool", "Configuration"] {
        let readme = README.replace(&format!("## {missing}\n\n"), "");
        let mut files = external("", COMPLEXIPY, vec![]);
        files[2].text = readme;

        let message = issues(vec![files]);

        assert!(
            message.contains(&format!("missing section `{missing}`")),
            "{missing}: {message}"
        );
    }
}

#[test]
fn the_readme_of_an_external_rule_may_have_exceptions_and_options_in_their_places() {
    let full = README
        .replace("## Tool", "## Exceptions\n\ne\n\n## Tool")
        .replace("## Suppression", "## Options\n\no\n\n## Suppression");
    let mut files = external("", COMPLEXIPY, vec![]);
    files[2].text = full;
    assert!(catalog(vec![files]).is_ok());

    let misplaced = README.replace("## Why", "## Tool\n\nt\n\n## Why");
    let mut files = external("", COMPLEXIPY, vec![]);
    files[2].text = misplaced.replacen("## Tool\n\nt\n\n## Configuration", "## Configuration", 1);
    let message = issues(vec![files]);
    assert!(
        message.contains("section `Why` comes after `Tool`"),
        "{message}"
    );
}
