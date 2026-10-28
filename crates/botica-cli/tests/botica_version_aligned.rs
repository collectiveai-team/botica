//! `botica-version-aligned` in `botica lint`: the run reads `prek.toml`, `.pre-commit-config.yaml`,
//! the workflows and the configuration, and compares their versions of botica with the mirror.

mod common;

use common::{botica, project_in, stdout, write};

const SELECTED: &str = "select = [\"botica-version-aligned\"]\npreview = true\n";
const PREK: &str = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"commits\"\n\
                    entry = \"uvx botica@0.16.1 lint --select conventional-commits --commit-msg\"\n\n\
                    [[repos]]\nrepo = \"https://github.com/collectiveai-team/botica-pre-commit\"\n\
                    rev = \"v0.17.1\"\n";
const WORKFLOW: &str = "jobs:\n  a:\n    steps:\n      - run: uvx botica@0.16.1 lint\n";

#[test]
fn a_hook_and_a_workflow_behind_the_mirror_are_reported_at_their_lines() {
    let dir = project_in("botica-version-aligned", "behind", Some(SELECTED));
    write(&dir, "prek.toml", PREK);
    write(&dir, ".github/workflows/abide.yml", WORKFLOW);

    let text = stdout(&botica(&dir, &[]));

    assert!(
        text.contains(
            "prek.toml:6:14: warning [botica-version-aligned] This line runs botica 0.16.1, and \
             the `rev` of the mirror"
        ),
        "{text}"
    );
    assert!(
        text.contains(".github/workflows/abide.yml:4:18: warning [botica-version-aligned]"),
        "{text}"
    );

    write(&dir, "prek.toml", &PREK.replace("@0.16.1", "@0.17.1"));
    write(
        &dir,
        ".github/workflows/abide.yml",
        &WORKFLOW.replace("@0.16.1", "@0.17.1"),
    );
    assert!(!stdout(&botica(&dir, &[])).contains("botica-version-aligned"));
}

#[test]
fn a_workflow_in_extend_ignores_is_not_read() {
    let config = format!(
        "{SELECTED}[rules.botica-version-aligned]\nextend-ignores = [\".github/workflows/**\"]\n"
    );
    let dir = project_in("botica-version-aligned", "ignored", Some(&config));
    write(&dir, "prek.toml", &PREK.replace("@0.16.1", "@0.17.1"));
    write(&dir, ".github/workflows/abide.yml", WORKFLOW);

    assert!(!stdout(&botica(&dir, &[])).contains("botica-version-aligned"));
}

#[test]
fn the_parameter_botica_version_of_pyproject_toml_is_reported_at_its_value() {
    let dir = project_in("botica-version-aligned", "pyproject", None);
    write(
        &dir,
        "pyproject.toml",
        "[project]\nname = \"demo\"\n\n[tool.botica]\nselect = [\"botica-version-aligned\"]\n\
         preview = true\n\n[tool.botica.recipes.params]\nbotica-version = \"0.16.1\"\n",
    );
    write(&dir, "prek.toml", &PREK.replace("@0.16.1", "@0.17.1"));

    let text = stdout(&botica(&dir, &[]));

    assert!(
        text.contains(
            "pyproject.toml:9:18: warning [botica-version-aligned] The parameter \
             `botica-version = \"0.16.1\"` of the recipes makes the next `botica recipe install` \
             write `botica@0.16.1` again"
        ),
        "{text}"
    );
    assert_eq!(text.matches("botica-version-aligned]").count(), 1, "{text}");
}

#[test]
fn the_mirror_of_pre_commit_is_the_reference_when_prek_toml_has_none() {
    let dir = project_in("botica-version-aligned", "pre-commit", Some(SELECTED));
    write(
        &dir,
        ".pre-commit-config.yaml",
        "repos:\n  - repo: https://github.com/collectiveai-team/botica-pre-commit\n    \
         rev: v0.17.1\n    hooks:\n      - id: botica\n",
    );
    write(&dir, ".github/workflows/abide.yml", WORKFLOW);

    let text = stdout(&botica(&dir, &[]));

    assert!(
        text.contains(
            ".github/workflows/abide.yml:4:18: warning [botica-version-aligned] This line runs \
             botica 0.16.1, and the `rev` of the mirror `collectiveai-team/botica-pre-commit` in \
             .pre-commit-config.yaml is `v0.17.1`"
        ),
        "{text}"
    );
}
