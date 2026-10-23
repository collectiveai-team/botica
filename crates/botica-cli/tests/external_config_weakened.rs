//! `external-config-weakened` in `botica lint --scope project`: a value of the settings of a tool
//! that holds less than an active external rule asks, silenced by a comment with a reason.

mod common;

use std::path::PathBuf;

use common::{botica, project_in, stdout, write};

const CONFIG: &str = "preview = true\n\
                      extend-select = [\"acme/pyrefly-strict\", \"external-config-weakened\"]\n\
                      ignore = [\"docstring-matches-signature\"]\n";

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
                      ## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// A project whose catalog `acme` has a rule that asks pyrefly for `preset = "strict"`, and whose
/// `pyproject.toml` is `pyproject`.
fn project(name: &str, config: &str, pyproject: &str) -> PathBuf {
    let dir = project_in("external-config-weakened", name, Some(config));
    let rule = ".botica/rules/acme/checks/pyrefly-strict";
    write(&dir, ".botica/rules/acme/catalog.toml", "code = \"ACME\"\n");
    write(
        &dir,
        ".botica/rules/acme/checks/family.toml",
        "code = \"CHK\"\ndescription = \"The checks that another tool runs\"\n",
    );
    write(
        &dir,
        &format!("{rule}/rule.toml"),
        "id = \"acme/pyrefly-strict\"\ncode = \"ACMECHK001\"\nsummary = \"s\"\n\
         status = \"preview\"\ndefault-enabled = false\ndefault-severity = \"error\"\n\n\
         [[implementations]]\nkind = \"external\"\nlanguages = [\"python\"]\ntool = \"pyrefly\"\n\
         docs = \"https://pyrefly.org/en/docs/configuration/\"\nconfig = \"config/pyproject.toml\"\n",
    );
    write(&dir, &format!("{rule}/README.md"), README);
    write(
        &dir,
        &format!("{rule}/config/pyproject.toml"),
        "[tool.pyrefly]\npreset = \"strict\"\n",
    );
    write(&dir, "app.py", "print(1)\n");
    write(&dir, "pyproject.toml", pyproject);
    dir
}

#[test]
fn a_weaker_preset_is_a_warning_on_its_line_that_does_not_fail_the_run() {
    let dir = project(
        "basic",
        CONFIG,
        "[project]\nname = \"app\"\n\n[tool.pyrefly]\npreset = \"basic\"\n",
    );

    let output = botica(&dir, &["--scope", "project"]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(
        text.contains(
            "pyproject.toml:5:1: warning [external-config-weakened] `preset = \"basic\"` in \
             pyproject.toml weakens what `acme/pyrefly-strict` asks for (`strict`)"
        ),
        "{text}"
    );
    assert!(!text.contains("[external-config]"), "{text}");
}

#[test]
fn a_comment_with_a_reason_silences_it_and_one_that_silences_nothing_is_reported() {
    let silenced = project(
        "silenced",
        CONFIG,
        "[tool.pyrefly]\npreset = \"basic\"  # botica: ignore[external-config-weakened] -- moves to strict in Q1\n",
    );
    assert!(!stdout(&botica(&silenced, &["--scope", "project"])).contains("pyproject.toml"));

    let stale = project(
        "stale",
        CONFIG,
        "[tool.pyrefly]\npreset = \"strict\"  # botica: ignore[external-config-weakened] -- moves to strict in Q1\n",
    );
    // `--scope project` leaves out `unused-suppression`, a rule of file scope: the run of every
    // scope reports the comment.
    let text = stdout(&botica(&stale, &[]));
    assert!(
        text.contains("pyproject.toml:2:22: warning [unused-suppression]"),
        "{text}"
    );
}

#[test]
fn the_walk_reports_a_comment_without_a_reason_and_accepts_one_with_it() {
    let dir = project(
        "no-reason",
        CONFIG,
        "[tool.pyrefly]\npreset = \"basic\"  # botica: ignore[external-config-weakened]\n",
    );

    let text = stdout(&botica(&dir, &["--scope", "file", "pyproject.toml"]));

    assert!(
        text.contains(
            "warning [invalid-suppression] A suppression of `external-config-weakened` needs a \
             reason after `--`"
        ),
        "{text}"
    );
}

#[test]
fn the_configuration_switches_it_off() {
    let config = format!("{CONFIG}ignore = [\"external-config-weakened\"]\n");
    let dir = project("off", &config, "[tool.pyrefly]\npreset = \"basic\"\n");

    assert!(!stdout(&botica(&dir, &["--scope", "project"])).contains("external-config-weakened"));
}
