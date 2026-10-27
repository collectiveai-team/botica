//! A rule with `python` is active only in a project whose `requires-python` allows, as its lowest
//! version, one inside the range (ADR 0015).

mod common;

use std::path::{Path, PathBuf};

use common::{botica, command, project_in, stdout, write};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

const SELECTS_BOTH: &str =
    "preview = true\nextend-select = [\"acme/before-315\", \"acme/from-315\"]\n";

/// A project with `a.py`, a `pyproject.toml` with `requires_python` (none when `None`) and the
/// catalog `acme`: two external rules of ruff, `acme/before-315` for `<3.15` and `acme/from-315`
/// for `>=3.15`, each asking for one code under `select`.
fn project(name: &str, requires_python: Option<&str>) -> PathBuf {
    let dir = project_in("rules-python", name, Some(SELECTS_BOTH));
    write(&dir, "a.py", "x = 1\n");
    let requires = requires_python
        .map(|range| format!("requires-python = \"{range}\"\n"))
        .unwrap_or_default();
    write(
        &dir,
        "pyproject.toml",
        &format!("[project]\nname = \"a\"\n{requires}\n[tool.ruff.lint]\nselect = [\"E\"]\n"),
    );
    let acme = ".botica/rules/acme";
    write(&dir, &format!("{acme}/catalog.toml"), "code = \"ACME\"\n");
    write(
        &dir,
        &format!("{acme}/checks/family.toml"),
        "code = \"CHK\"\ndescription = \"The checks that another tool runs\"\n",
    );
    for (slug, code, range, ruff) in [
        ("before-315", "ACMECHK001", "<3.15", "TID253"),
        ("from-315", "ACMECHK002", ">=3.15", "PLC0415"),
    ] {
        let rule = format!("{acme}/checks/{slug}");
        write(
            &dir,
            &format!("{rule}/rule.toml"),
            &format!(
                "id = \"acme/{slug}\"\ncode = \"{code}\"\nsummary = \"s\"\nstatus = \"preview\"\n\
                 default-enabled = false\ndefault-severity = \"warning\"\npython = \"{range}\"\n\n\
                 [[implementations]]\nkind = \"external\"\nlanguages = [\"python\"]\n\
                 tool = \"ruff\"\ndocs = \"https://docs.astral.sh/ruff/rules/\"\n\
                 codes = [\"{ruff}\"]\nconfig = \"config/pyproject.toml\"\n"
            ),
        );
        write(&dir, &format!("{rule}/README.md"), README);
        write(
            &dir,
            &format!("{rule}/config/pyproject.toml"),
            &format!("[tool.ruff.lint]\nselect = [\"{ruff}\"]\n"),
        );
    }
    dir
}

fn line_of<'a>(text: &'a str, slug: &str) -> &'a str {
    text.lines()
        .find(|line| line.split_whitespace().any(|word| word == slug))
        .unwrap_or_else(|| panic!("no line for {slug} in:\n{text}"))
}

fn rules_all(dir: &Path) -> String {
    let output = command(dir, &["rules", "--all"]);
    assert!(output.status.success(), "{output:?}");
    stdout(&output)
}

#[test]
fn a_project_before_315_runs_the_rule_for_it_and_says_why_the_other_is_not_active() {
    let text = rules_all(&project("before", Some(">=3.12, <4")));
    assert!(
        line_of(&text, "acme/before-315").contains(" active "),
        "{text}"
    );
    let from = line_of(&text, "acme/from-315");
    assert!(
        from.contains("inactive")
            && from.contains("requires Python `>=3.15`, and the project requires `>=3.12, <4`"),
        "{from}"
    );
}

#[test]
fn a_project_from_315_runs_the_other_rule() {
    let text = rules_all(&project("from", Some(">=3.15")));
    assert!(
        line_of(&text, "acme/from-315").contains(" active "),
        "{text}"
    );
    assert!(
        line_of(&text, "acme/before-315")
            .contains("requires Python `<3.15`, and the project requires `>=3.15`"),
        "{text}"
    );
}

#[test]
fn a_project_without_requires_python_runs_the_rule_for_the_older_range() {
    let text = rules_all(&project("unpinned", None));
    assert!(
        line_of(&text, "acme/before-315").contains(" active "),
        "{text}"
    );
    assert!(
        line_of(&text, "acme/from-315").contains(
            "requires Python `>=3.15`, and the project names no lower bound in `requires-python`"
        ),
        "{text}"
    );
}

#[test]
fn external_config_asks_only_for_the_fragment_of_the_rule_that_matches() {
    let output = botica(
        &project("lint", Some(">=3.15")),
        &["--output-format", "json", "."],
    );
    let text = stdout(&output);
    assert!(text.contains("`acme/from-315`"), "{text}");
    assert!(!text.contains("`acme/before-315`"), "{text}");
}

#[test]
fn recipe_check_counts_a_rule_outside_the_python_of_the_project_as_ok() {
    let dir = project("recipe", Some(">=3.12"));
    write(
        &dir,
        "pyproject.toml",
        "[project]\nname = \"a\"\nrequires-python = \">=3.12\"\n\n\
         [tool.ruff.lint]\nselect = [\"TID253\"]\n",
    );
    write(
        &dir,
        "recipes/heavy.toml",
        "description = \"d\"\nrules = [\"acme/before-315\", \"acme/from-315\"]\n",
    );
    let output = command(&dir, &["recipe", "check", "heavy"]);
    let text = stdout(&output);
    assert!(output.status.success(), "{text}");
    assert!(text.contains("rule acme/before-315: ok\n"), "{text}");
    assert!(
        text.contains(
            "rule acme/from-315: ok, not active: requires Python `>=3.15`, and the project \
             requires `>=3.12`"
        ),
        "{text}"
    );
    assert!(
        text.contains("fragment pyproject.toml `acme/before-315`: ok"),
        "{text}"
    );
    assert!(
        !text.contains("fragment pyproject.toml `acme/from-315`"),
        "{text}"
    );
}
