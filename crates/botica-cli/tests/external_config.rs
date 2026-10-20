//! `external-config` in `botica lint`: the run checks the configuration of the active external
//! rules once for the whole project, whatever files it is given.

mod common;

use common::{botica, project_in, stdout, write};

const SELECTS_COGNITIVE_COMPLEXITY: &str =
    "preview = true\nextend-select = [\"cognitive-complexity\"]\n";

const WITHOUT_THE_HOOK: &str = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"deptry\"\n";

const WITH_THE_HOOK: &str = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"complexipy\"\n\
entry = \"uvx complexipy --max-complexity-allowed 15\"\n";

const FINDING: &str = "prek.toml:1:1: error [external-config] prek.toml lacks the configuration \
of complexipy that `cognitive-complexity` needs: `botica explain cognitive-complexity` \
shows it";

#[test]
fn a_file_of_configuration_without_the_fragment_of_an_active_external_rule_is_reported() {
    let dir = project_in(
        "external-config",
        "lacks",
        Some(SELECTS_COGNITIVE_COMPLEXITY),
    );
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);
    write(&dir, "a.py", "x = 1\n");

    let output = botica(&dir, &["a.py"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).contains(FINDING), "{}", stdout(&output));
}

#[test]
fn a_file_with_the_fragment_has_no_finding() {
    let dir = project_in(
        "external-config",
        "holds",
        Some(SELECTS_COGNITIVE_COMPLEXITY),
    );
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
            "preview = true\nextend-select = [\"cognitive-complexity\"]\nignore = [\"external-config\"]\n",
        ),
    ] {
        let dir = project_in("external-config", name, Some(config));
        write(&dir, "prek.toml", WITHOUT_THE_HOOK);
        write(&dir, "a.py", "x = 1\n");

        let output = botica(&dir, &["a.py"]);

        assert_eq!(output.status.code(), Some(0), "{name}: {}", stdout(&output));
    }
}

#[test]
fn the_file_is_read_at_the_root_of_the_project_and_shown_from_where_botica_runs() {
    let dir = project_in(
        "external-config",
        "below",
        Some(SELECTS_COGNITIVE_COMPLEXITY),
    );
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
    let dir = project_in(
        "external-config",
        "scope",
        Some(SELECTS_COGNITIVE_COMPLEXITY),
    );
    write(&dir, "prek.toml", WITHOUT_THE_HOOK);

    let project = botica(&dir, &["--scope", "project"]);
    assert_eq!(project.status.code(), Some(1));
    assert!(stdout(&project).contains(FINDING), "{}", stdout(&project));

    let file = botica(&dir, &["--scope", "file"]);
    assert_eq!(file.status.code(), Some(0), "{}", stdout(&file));
}
