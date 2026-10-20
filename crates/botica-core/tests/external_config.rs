//! `external-config`: the configuration that an active external rule declares is in the file of
//! the project it goes into.

use std::collections::HashMap;

use botica_catalog::Catalog;
use botica_core::external_config::{check, Requirement, EXTERNAL_CONFIG};
use botica_core::Severity;

const HOOK: &str = "[[repos]]\n[[repos.hooks]]\nid = \"complexipy\"\n";

fn requirement(fragment: &str) -> Requirement<'_> {
    Requirement {
        rule: "cognitive-complexity",
        tool: "complexipy",
        target: "prek.toml",
        fragment,
    }
}

/// Runs the check against files held in memory.
fn findings(
    requirements: &[Requirement<'_>],
    files: &[(&str, &str)],
) -> Vec<botica_core::Diagnostic> {
    let files: HashMap<&str, &str> = files.iter().copied().collect();
    check(
        requirements,
        |path| files.get(path).map(|text| (*text).to_owned()),
        Severity::Error,
    )
}

#[test]
fn a_file_that_holds_the_fragment_among_other_settings_has_no_finding() {
    let prek = "[[repos]]\nrepo = \"https://github.com/astral-sh/uv-pre-commit\"\n\
                hooks = [{ id = \"uv-lock\" }]\n\n[[repos]]\nrepo = \"local\"\n\n\
                [[repos.hooks]]\nid = \"deptry\"\n\n[[repos.hooks]]\nid = \"complexipy\"\n\
                entry = \"uvx complexipy --max-complexity-allowed 15\"\n";

    assert!(findings(&[requirement(HOOK)], &[("prek.toml", prek)]).is_empty());
}

#[test]
fn a_file_without_the_fragment_has_a_finding_about_the_whole_file() {
    let prek = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"deptry\"\n";

    let found = findings(&[requirement(HOOK)], &[("prek.toml", prek)]);

    assert_eq!(found.len(), 1);
    let finding = &found[0];
    assert_eq!(finding.path, "prek.toml");
    assert_eq!(finding.rule, EXTERNAL_CONFIG);
    assert_eq!(finding.severity, Severity::Error);
    assert!(finding.file_level);
    assert_eq!(
        finding.message,
        "prek.toml lacks the configuration of complexipy that `cognitive-complexity` \
         needs: `botica explain cognitive-complexity` shows it"
    );
}

#[test]
fn a_missing_file_or_one_that_is_not_toml_has_a_finding() {
    let missing = findings(&[requirement(HOOK)], &[]);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].path, "prek.toml");
    assert_eq!(
        missing[0].message,
        "prek.toml does not exist, and complexipy needs it for `cognitive-complexity`: \
         `botica explain cognitive-complexity` shows what goes in it"
    );

    let broken = findings(&[requirement(HOOK)], &[("prek.toml", "[[repos]\n")]);
    assert_eq!(broken.len(), 1);
    assert!(
        broken[0].message.starts_with(
            "prek.toml is not TOML, so botica cannot find the configuration of \
                          complexipy that `cognitive-complexity` needs: "
        ),
        "{}",
        broken[0].message
    );
}

#[test]
fn a_table_holds_a_fragment_when_it_has_each_key_with_a_value_that_holds_the_fragment() {
    let cases = [
        ("[tool.ruff.lint]\nselect = [\"C90\"]\n", true),
        ("[tool.ruff.lint]\nselect = [\"C91\"]\n", false),
        ("[tool.ruff.lint]\nselect = [\"C90\", \"C91\"]\n", false),
        (
            "[tool.ruff.lint]\nselect = [\"C90\"]\nignore = [\"E999\"]\n",
            false,
        ),
        ("[tool.ruff.lint.mccabe]\nmax-complexity = 10\n", true),
        ("[tool.ruff.lint.mccabe]\nmax-complexity = 12\n", false),
        ("[tool.ruff]\nline-length = 100\n", false),
        (
            "[tool.ruff.lint]\nselect = [\"E\", \"C90\"]\nignore = [\"E501\"]\n",
            true,
        ),
    ];
    let pyproject = "[project]\nname = \"x\"\n\n[tool.ruff]\nline-length = 99\n\n\
                     [tool.ruff.lint]\nselect = [\"E\", \"F\", \"C90\"]\nignore = [\"E501\"]\n\n\
                     [tool.ruff.lint.mccabe]\nmax-complexity = 10\n";
    for (fragment, held) in cases {
        let mut needed = requirement(fragment);
        needed.target = "pyproject.toml";

        let found = findings(&[needed], &[("pyproject.toml", pyproject)]);

        assert_eq!(found.is_empty(), held, "{fragment}");
    }
}

#[test]
fn each_rule_that_lacks_its_configuration_has_a_finding_of_its_own() {
    let deptry = Requirement {
        rule: "dep-hygiene",
        tool: "deptry",
        target: "prek.toml",
        fragment: "[[repos]]\n[[repos.hooks]]\nid = \"deptry\"\n",
    };
    let prek = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"ruff\"\n";

    let found = findings(&[requirement(HOOK), deptry], &[("prek.toml", prek)]);

    let messages: Vec<&str> = found
        .iter()
        .map(|finding| finding.message.as_str())
        .collect();
    assert_eq!(found.len(), 2);
    assert!(
        messages[1].contains("of deptry that `dep-hygiene` needs:"),
        "{messages:?}"
    );
}

#[test]
fn the_requirements_of_a_rule_are_the_fragments_of_its_external_implementations() {
    let catalog = Catalog::embedded().expect("the catalog loads");
    let rule = catalog.resolve("cognitive-complexity").expect("the rule");

    let requirements = Requirement::of(rule);

    assert_eq!(requirements.len(), 1);
    let requirement = &requirements[0];
    assert_eq!(requirement.rule, "cognitive-complexity");
    assert_eq!(requirement.tool, "complexipy");
    assert_eq!(requirement.target, "prek.toml");
    assert_eq!(requirement.fragment, HOOK);

    let native = catalog.resolve("no-utils").expect("the rule");
    assert!(Requirement::of(native).is_empty());
}
