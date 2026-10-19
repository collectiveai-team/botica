//! A project adds its own rules in `.botica/rules/<catalog>/`.

mod common;

use common::{command, project_in, stderr, stdout, write};
use std::path::{Path, PathBuf};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

fn rule_toml(extra: &str) -> String {
    format!(
        "id = \"acme/no-eval\"\ncode = \"ACMESTRUC001\"\nsummary = \"Do not call eval\"\n\
         status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\n\
         default-enabled = true\ndefault-severity = \"error\"\n{extra}\n\n[[implementations]]\n\
         kind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n"
    )
}

fn yaml(pattern: &str) -> String {
    format!(
        "id: acme/no-eval\nlanguage: python\nseverity: error\nmessage: \"ACMESTRUC001 (acme/no-eval): do not call eval\"\nrule:\n  pattern: {pattern}\n"
    )
}

fn lay_out_acme(dir: &Path) {
    let root = ".botica/rules/acme";
    write(dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{root}/structure/no-eval");
    write(dir, &format!("{rule}/rule.toml"), &rule_toml(""));
    write(dir, &format!("{rule}/README.md"), README);
    write(dir, &format!("{rule}/python.yml"), &yaml("eval($$$ARGS)"));
    write(
        dir,
        &format!("{rule}/fixtures/invalid/bad.py"),
        "eval(\"1\")\n",
    );
}

fn project(name: &str, config: &str) -> PathBuf {
    let dir = project_in("project-catalogs", name, Some(config));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    lay_out_acme(&dir);
    write(&dir, "app.py", "eval(\"1\")\n");
    dir
}

/// The ids of the rules that reported, in the order of the output.
fn reported(dir: &Path, args: &[&str]) -> Vec<String> {
    let mut all = vec!["lint", "."];
    all.extend(args);
    stdout(&command(dir, &all))
        .lines()
        .filter_map(|line| line.split_once('[')?.1.split_once(']'))
        .map(|(rule, _)| rule.to_owned())
        .collect()
}

#[test]
fn a_rule_of_a_project_catalog_runs_and_reports_under_its_prefixed_id() {
    let dir = project("runs", "");

    let output = command(&dir, &["lint", "."]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}{}", stderr(&output));
    assert_eq!(reported(&dir, &[]), ["acme/no-eval"], "{text}");
    assert!(text.starts_with("app.py:1:"), "{text}");
}

#[test]
fn the_fixtures_of_a_project_catalog_are_not_linted() {
    let dir = project("fixtures", "");

    let text = stdout(&command(&dir, &["lint", "."]));

    assert!(!text.contains(".botica"), "{text}");
}

#[test]
fn a_rule_of_a_project_answers_to_its_id_and_its_code_on_the_command_line() {
    let dir = project("selected", "select = [\"no-utils\"]\n");

    assert!(reported(&dir, &[]).is_empty());
    for key in ["acme/no-eval", "ACMESTRUC001", "ACMESTRUC"] {
        assert_eq!(
            reported(&dir, &["--select", key]),
            ["acme/no-eval"],
            "{key}"
        );
        assert_eq!(
            reported(&dir, &["--extend-select", key]),
            ["acme/no-eval"],
            "{key}"
        );
    }
    let ignoring = project("ignored", "");
    for key in ["acme/no-eval", "ACMESTRUC001", "ACMESTRUC"] {
        assert!(reported(&ignoring, &["--ignore", key]).is_empty(), "{key}");
    }
}

#[test]
fn the_configuration_names_a_rule_of_a_project_by_its_quoted_id() {
    let dir = project("severity", "[rules.\"acme/no-eval\"]\nseverity = \"off\"\n");

    assert!(reported(&dir, &[]).is_empty());
}

#[test]
fn both_comments_silence_a_rule_of_a_project() {
    let dir = project("silenced", "");
    write(
        &dir,
        "app.py",
        "eval(1)  # noqa: ACMESTRUC001\neval(2)  # botica: ignore[acme/no-eval]\neval(3)\n",
    );

    let text = stdout(&command(&dir, &["lint", "."]));

    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.starts_with("app.py:3:"), "{text}");
}

#[test]
fn a_comment_that_names_an_unknown_rule_of_the_catalog_is_reported() {
    let dir = project("unknown", "");
    write(
        &dir,
        "app.py",
        "x = 1  # noqa: ACMENOPE001\ny = 2  # botica: ignore[acme/no-such]\n",
    );

    let text = stdout(&command(&dir, &["lint", "."]));

    assert_eq!(text.matches("invalid-suppression").count(), 2, "{text}");
}

#[test]
fn rules_lists_a_rule_of_a_project_with_its_code() {
    let dir = project("listed", "");

    let output = command(&dir, &["rules"]);

    let text = stdout(&output);
    let line = text
        .lines()
        .find(|line| line.contains("acme/no-eval"))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(line.contains("ACMESTRUC001"), "{line}");
}

#[test]
fn explain_describes_a_rule_of_a_project_by_id_or_by_code() {
    let dir = project("explained", "");

    for key in ["acme/no-eval", "ACMESTRUC001"] {
        let output = command(&dir, &["explain", key]);

        let text = stdout(&output);
        assert!(output.status.success(), "{key}: {}", stderr(&output));
        assert!(
            text.starts_with("acme/no-eval (ACMESTRUC001): Do not call eval"),
            "{key}: {text}"
        );
    }
}

#[test]
fn a_catalog_that_does_not_load_stops_the_command_with_the_file_and_the_problem() {
    let dir = project("broken", "");
    write(
        &dir,
        ".botica/rules/acme/structure/no-eval/README.md",
        "## Nothing\n",
    );

    for args in [&["lint", "."][..], &["rules"], &["explain", "no-utils"]] {
        let output = command(&dir, args);

        let message = stderr(&output);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {message}");
        assert!(
            message.contains("acme/structure/no-eval/README.md"),
            "{args:?}: {message}"
        );
    }
}

#[test]
fn a_project_with_no_rules_folder_has_only_the_built_in_rules() {
    let dir = project_in("project-catalogs", "none", Some(""));
    write(&dir, "app.py", "eval(\"1\")\n");

    let output = command(&dir, &["lint", "."]);

    assert!(output.status.success(), "{}", stderr(&output));
}

#[test]
fn the_rules_are_read_from_the_root_of_the_project_wherever_botica_runs() {
    let dir = project("subdir", "");
    write(&dir, "sub/app.py", "eval(\"1\")\n");

    let output = command(&dir.join("sub"), &["lint", "."]);

    assert!(
        stdout(&output).contains("[acme/no-eval]"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn the_cache_forgets_what_a_change_of_the_catalog_of_the_project_changes() {
    let dir = project("cache", "");
    let run = |dir: &Path| stdout(&command(dir, &["lint", ".", "--cache"]));

    assert!(run(&dir).contains("[acme/no-eval]"));
    // The YAML changes what the rule finds.
    write(
        &dir,
        ".botica/rules/acme/structure/no-eval/python.yml",
        &yaml("exec($$$ARGS)"),
    );
    assert!(!run(&dir).contains("[acme/no-eval]"));
}

#[test]
fn the_cache_forgets_what_a_new_alias_in_rule_toml_silences() {
    let dir = project("cache-alias", "");
    write(&dir, "app.py", "eval(1)  # botica: ignore[acme/old-name]\n");
    let run = |dir: &Path| stdout(&command(dir, &["lint", ".", "--cache"]));

    let before = run(&dir);
    assert!(before.contains("invalid-suppression"), "{before}");
    write(
        &dir,
        ".botica/rules/acme/structure/no-eval/rule.toml",
        &rule_toml("aliases = [\"acme/old-name\"]"),
    );

    assert_eq!(run(&dir), "");
}
