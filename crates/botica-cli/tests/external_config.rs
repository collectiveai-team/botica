//! `external-config` in `botica lint`: the run checks the configuration of the active external
//! rules once for the whole project, whatever files it is given.

mod common;

use std::path::PathBuf;

use common::external_rule::{
    complexity_rule, DOCS_RUFF, LACKS, SELECTS_COMPLEXITY, WITHOUT_THE_HOOK, WITH_THE_HOOK,
};
use common::{botica, project_in, stdout, write};

fn project(name: &str, config: &str) -> PathBuf {
    let dir = project_in("external-config", name, Some(config));
    complexity_rule(&dir);
    dir
}

fn finding() -> String {
    format!("prek.toml:1:1: error [external-config] {LACKS}")
}

#[test]
fn a_file_of_configuration_without_the_fragment_of_an_active_external_rule_is_reported() {
    let dir = project("lacks", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);
    write(&dir, "a.py", "x = 1\n");

    let output = botica(&dir, &["a.py"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains(&finding()), "{}", stdout(&output));
}

#[test]
fn a_file_with_the_fragment_has_no_finding() {
    let dir = project("holds", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITH_THE_HOOK);
    write(&dir, "a.py", "x = 1\n");

    let output = botica(&dir, &["a.py"]);

    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}

#[test]
fn an_external_rule_that_is_not_active_or_the_rule_switched_off_reports_nothing() {
    for (name, config) in [
        ("inactive", "preview = true\n"),
        (
            "ignored",
            "preview = true\nextend-select = [\"acme/complexity\"]\nignore = [\"external-config\"]\n",
        ),
    ] {
        let dir = project(name, config);
        write(&dir, "prek.toml", WITHOUT_THE_HOOK);
        write(&dir, "a.py", "x = 1\n");

        let output = botica(&dir, &["a.py"]);

        assert_eq!(output.status.code(), Some(0), "{name}: {}", stdout(&output));
    }
}

#[test]
fn the_file_is_read_at_the_root_of_the_project_and_shown_from_where_botica_runs() {
    let dir = project("below", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);
    write(&dir, "src/app/a.py", "x = 1\n");

    let output = botica(&dir.join("src/app"), &["a.py"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("../../prek.toml:1:1: error [external-config] prek.toml lacks"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn the_rules_of_the_project_scope_check_the_external_rules_that_are_selected() {
    let dir = project("scope", SELECTS_COMPLEXITY);
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);

    let project = botica(&dir, &["--scope", "project"]);
    assert_eq!(project.status.code(), Some(1));
    assert!(
        stdout(&project).contains(&finding()),
        "{}",
        stdout(&project)
    );

    let file = botica(&dir, &["--scope", "file"]);
    assert_eq!(file.status.code(), Some(0), "{}", stdout(&file));
}

/// A project whose catalog has `acme/selection`, which needs ruff to select `ASYNC`, and selects it
/// alone of the rules of ruff.
fn ruff_project(name: &str) -> PathBuf {
    let dir = project_in(
        "external-config",
        name,
        Some(
            "preview = true\nextend-select = [\"acme/selection\"]\n\
             ignore = [\"docstring-matches-signature\"]\n",
        ),
    );
    complexity_rule(&dir);
    let rule = ".botica/rules/acme/checks/selection";
    write(
        &dir,
        &format!("{rule}/rule.toml"),
        &format!(
            "id = \"acme/selection\"\ncode = \"ACMECHK002\"\n\
             summary = \"ruff checks async code\"\nstatus = \"preview\"\n\
             default-enabled = false\ndefault-severity = \"error\"\n\n\
             [[implementations]]\nkind = \"external\"\nlanguages = [\"python\"]\n\
             tool = \"ruff\"\ndocs = \"{DOCS_RUFF}\"\nconfig = \"config/pyproject.toml\"\n"
        ),
    );
    write(
        &dir,
        &format!("{rule}/README.md"),
        "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
         ## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n",
    );
    write(
        &dir,
        &format!("{rule}/config/pyproject.toml"),
        "[tool.ruff.lint]\nselect = [\"ASYNC\"]\n",
    );
    write(&dir, "a.py", "x = 1\n");
    dir
}

#[test]
fn the_settings_of_ruff_are_read_where_ruff_reads_them() {
    let in_ruff_toml = ruff_project("ruff-toml");
    write(&in_ruff_toml, "pyproject.toml", "[project]\nname = \"x\"\n");
    write(
        &in_ruff_toml,
        "ruff.toml",
        "[lint]\nextend-select = [\"ASYNC\"]\n",
    );
    let output = botica(&in_ruff_toml, &["a.py"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));

    let ignored = ruff_project("ignored-family");
    write(
        &ignored,
        "pyproject.toml",
        "[tool.ruff.lint]\nselect = [\"E\"]\nignore = [\"ASYNC\"]\n",
    );
    let output = botica(&ignored, &["a.py"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));

    let lacks = ruff_project("lacks-family");
    write(
        &lacks,
        "pyproject.toml",
        "[tool.ruff.lint]\nselect = [\"E\"]\n",
    );
    write(&lacks, "ruff.toml", "[lint]\nselect = [\"F\"]\n");
    let output = botica(&lacks, &["a.py"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains(
            "ruff.toml:1:1: error [external-config] ruff.toml lacks the configuration of ruff \
             that `acme/selection` needs"
        ),
        "{}",
        stdout(&output)
    );
}

/// A project whose catalog has `acme/cooldown`, which needs pnpm to wait 7 days before it installs
/// a release, and applies when the project has `pnpm-lock.yaml`.
fn pnpm_project(name: &str) -> PathBuf {
    let dir = project_in(
        "external-config",
        name,
        Some("preview = true\nextend-select = [\"acme/cooldown\"]\n"),
    );
    complexity_rule(&dir);
    let rule = ".botica/rules/acme/checks/cooldown";
    write(
        &dir,
        &format!("{rule}/rule.toml"),
        "id = \"acme/cooldown\"\ncode = \"ACMECHK003\"\n\
         summary = \"pnpm installs releases at least 7 days old\"\nstatus = \"preview\"\n\
         default-enabled = false\ndefault-severity = \"error\"\n\n\
         [[implementations]]\nkind = \"external\"\nlanguages = [\"javascript\", \"typescript\"]\n\
         tool = \"pnpm\"\ndocs = \"https://pnpm.io/settings\"\n\
         config = \"config/pnpm-workspace.yaml\"\napplies-when = [\"pnpm-lock.yaml\"]\n",
    );
    write(
        &dir,
        &format!("{rule}/README.md"),
        "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
         ## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n",
    );
    write(
        &dir,
        &format!("{rule}/config/pnpm-workspace.yaml"),
        "minimumReleaseAge: 10080\n",
    );
    dir
}

#[test]
fn an_external_rule_asks_nothing_of_a_project_that_does_not_use_its_tool() {
    let without_the_lockfile = pnpm_project("no-lockfile");
    let output = botica(&without_the_lockfile, &["a.py"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));

    let with_the_lockfile = pnpm_project("lockfile");
    write(
        &with_the_lockfile,
        "pnpm-lock.yaml",
        "lockfileVersion: '9.0'\n",
    );
    let output = botica(&with_the_lockfile, &["a.py"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains(
            "pnpm-workspace.yaml:1:1: error [external-config] pnpm-workspace.yaml does not exist, \
             and pnpm needs it for `acme/cooldown`"
        ),
        "{}",
        stdout(&output)
    );
}

#[test]
fn a_value_that_the_project_sets_weaker_on_purpose_is_not_reported() {
    let dir = pnpm_project("weaker");
    write(&dir, "pnpm-lock.yaml", "lockfileVersion: '9.0'\n");
    write(&dir, "pnpm-workspace.yaml", "minimumReleaseAge: 0\n");

    let output = botica(&dir, &["a.py"]);

    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
}
