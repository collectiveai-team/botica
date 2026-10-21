//! `botica recipe show`, `check` and `install` for the `tools` step: packages that `uv add` adds to
//! `pyproject.toml` when it does not declare them. The tests put a `uv` of their own first on the
//! `PATH`.
#![cfg(unix)]

mod common;

use std::path::PathBuf;

use common::recipes::{add_recipe, project, recipe, uv_calls, with_uv, without_npx, UV_ADDS};
use common::refs::{failed, ok};
use common::write;

const TOOLS: &str = "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\", \"pyrefly\"]\n\n\
                     [[tools]]\nmanager = \"uv\"\ngroup = \"docs\"\npackages = [\"zensical\"]\n";
const PYPROJECT: &str = "[project]\nname = \"demo\"\n";

fn setup(name: &str, toml: &str) -> PathBuf {
    let dir = project(name, None);
    add_recipe(&dir, "mine", toml);
    dir
}

#[test]
fn show_and_list_say_which_packages_a_recipe_adds() {
    let dir = setup("tool-show", TOOLS);
    add_recipe(
        &dir,
        "child",
        "extends = [\"mine\"]\n[[tools]]\nmanager = \"uv\"\npackages = [\"pytest\"]\n",
    );

    let shown = ok(&recipe(&dir, &["show", "child"]));
    let listed = ok(&recipe(&dir, &["list"]));

    assert!(
        shown.contains(
            "\n## Tools\n\n- ruff (uv, group dev, from mine)\n- pyrefly (uv, group dev, from mine)\n\
             - zensical (uv, group docs, from mine)\n- pytest (uv, group dev, from child)\n"
        ),
        "{shown}"
    );
    assert!(listed.contains("  tools: 3\n"), "{listed}");
    assert!(listed.contains("  tools: 1\n"), "{listed}");
}

#[test]
fn the_missing_packages_of_a_group_are_added_with_one_call() {
    let dir = setup("tool-add", TOOLS);
    write(&dir, "pyproject.toml", PYPROJECT);

    let text = ok(&with_uv(&dir, UV_ADDS, &["install", "mine"]));

    assert_eq!(
        text,
        "tool ruff: added\ntool pyrefly: added\ntool zensical: added\n\
         mine: 3 added, 0 ok, 0 pending\n"
    );
    assert_eq!(
        uv_calls(&dir),
        ["add --dev ruff pyrefly", "add --group docs zensical"]
    );
}

#[test]
fn a_package_that_pyproject_declares_is_ok_and_is_not_asked_for() {
    let dir = setup("tool-some", TOOLS);
    write(
        &dir,
        "pyproject.toml",
        "[dependency-groups]\ndev = [\"Ruff>=0.5\"]\n",
    );

    let text = ok(&with_uv(&dir, UV_ADDS, &["install", "mine"]));

    assert!(
        text.starts_with("tool ruff: ok\ntool pyrefly: added\ntool zensical: added\n"),
        "{text}"
    );
    assert_eq!(
        uv_calls(&dir),
        ["add --dev pyrefly", "add --group docs zensical"]
    );
}

#[test]
fn nothing_is_run_when_every_package_is_declared() {
    let dir = setup("tool-all", TOOLS);
    write(
        &dir,
        "pyproject.toml",
        "[dependency-groups]\ndev = [\"ruff\", \"pyrefly\"]\ndocs = [\"zensical\"]\n",
    );

    let text = ok(&without_npx(&dir, &["install", "mine"]));

    assert!(text.ends_with("mine: 0 added, 3 ok, 0 pending\n"), "{text}");
}

#[test]
fn a_package_is_declared_wherever_pyproject_lists_requirements() {
    let dir = setup(
        "tool-places",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"a-one\", \"b-two\", \"c-three\", \"d-four\", \"e-five\", \"f-six\"]\n",
    );
    write(
        &dir,
        "pyproject.toml",
        "[project]\nname = \"demo\"\ndependencies = [\"a_one>=1\"]\n\n\
         [project.optional-dependencies]\nextra = [\"B.Two[x] ; python_version > '3'\"]\n\n\
         [dependency-groups]\ntest = [\"c-three\", {include-group = \"d-four\"}]\n\n\
         [tool.uv]\ndev-dependencies = [\"e-five==2\"]\n",
    );

    let text = failed(&without_npx(&dir, &["install", "mine"]), 1);

    for declared in ["a-one", "b-two", "c-three", "e-five"] {
        assert!(text.contains(&format!("tool {declared}: ok\n")), "{text}");
    }
    for missing in ["d-four", "f-six"] {
        assert!(
            text.contains(&format!("tool {missing}: failed: cannot run `uv`: ")),
            "{text}"
        );
    }
}

#[test]
fn a_project_without_pyproject_leaves_the_packages_pending() {
    let dir = setup("tool-no-pyproject", TOOLS);
    let text = failed(&with_uv(&dir, UV_ADDS, &["install", "mine"]), 1);
    assert!(
        text.contains("tool ruff: pending: no pyproject.toml\n"),
        "{text}"
    );
    assert!(text.ends_with("mine: 0 added, 0 ok, 3 pending\n"), "{text}");
    assert!(uv_calls(&dir).is_empty());
    assert!(!dir.join("pyproject.toml").exists());
}

#[test]
fn a_pyproject_that_is_not_toml_stops_the_command_before_any_step() {
    let dir = setup("tool-broken", TOOLS);
    write(&dir, "pyproject.toml", "this is = not [ toml");

    let text = failed(&with_uv(&dir, UV_ADDS, &["install", "mine"]), 2);

    assert!(text.contains("pyproject.toml"), "{text}");
    assert!(uv_calls(&dir).is_empty());
}

#[test]
fn a_call_that_fails_is_reported_with_its_reason_and_the_other_groups_still_run() {
    let dir = setup("tool-fails", TOOLS);
    write(&dir, "pyproject.toml", PYPROJECT);
    let script = format!(
        "case \"$*\" in *zensical*) echo noise >&2; echo ' boom ' >&2; exit 2;; esac\n{UV_ADDS}"
    );

    let text = failed(&with_uv(&dir, &script, &["install", "mine"]), 1);

    assert!(
        text.contains("tool zensical: failed: `uv add --group docs` exited with code 2: boom\n"),
        "{text}"
    );
    assert!(text.contains("tool ruff: added\n"), "{text}");
    assert!(
        text.ends_with("mine: 2 added, 0 ok, 0 pending, 1 failed\n"),
        "{text}"
    );
}

/// A `uv` that fails, as the real one does, when a package of the call does not exist: nothing is
/// added, and the error is on stderr with a `hint:` as its last line.
const UV_REJECTS_BAD: &str = "case \"$*\" in *bad*)\n  echo \"$*\" >> \"$PWD/.uv-calls\"\n  \
    echo 'error: Failed to add dependencies' >&2\n  echo '  cause: No solution found' >&2\n  \
    echo '  cause: Because bad was not found' >&2\n  echo 'hint: use --frozen' >&2\n  exit 1;;\nesac";

#[test]
fn one_package_that_does_not_exist_does_not_fail_the_others_of_its_group() {
    let dir = setup(
        "tool-one-bad",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\", \"bad\", \"pytest\"]\n",
    );
    write(&dir, "pyproject.toml", PYPROJECT);
    let script = format!("{UV_REJECTS_BAD}\n{UV_ADDS}");

    let text = failed(&with_uv(&dir, &script, &["install", "mine"]), 1);

    assert_eq!(
        text,
        "tool ruff: added\n\
         tool bad: failed: `uv add --dev` exited with code 1: cause: Because bad was not found\n\
         tool pytest: added\nmine: 2 added, 0 ok, 0 pending, 1 failed\n"
    );
    assert_eq!(
        uv_calls(&dir),
        [
            "add --dev ruff bad pytest",
            "add --dev ruff",
            "add --dev bad",
            "add --dev pytest"
        ]
    );
}

#[test]
fn a_call_with_two_packages_is_retried_for_each() {
    let dir = setup(
        "tool-two",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\", \"bad\"]\n",
    );
    write(&dir, "pyproject.toml", PYPROJECT);
    let script = format!("{UV_REJECTS_BAD}\n{UV_ADDS}");

    failed(&with_uv(&dir, &script, &["install", "mine"]), 1);

    assert_eq!(
        uv_calls(&dir),
        ["add --dev ruff bad", "add --dev ruff", "add --dev bad"]
    );
}

#[test]
fn a_blank_last_line_or_a_word_that_starts_like_a_hint_is_not_skipped() {
    let dir = setup(
        "tool-blank",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\"]\n",
    );
    write(&dir, "pyproject.toml", PYPROJECT);

    let blank = "echo 'the reason' >&2\necho '   ' >&2\nexit 7";
    let text = failed(&with_uv(&dir, blank, &["install", "mine"]), 1);
    assert!(text.contains("exited with code 7: the reason\n"), "{text}");

    let word = "echo 'hinterland unreachable' >&2\nexit 8";
    let text = failed(&with_uv(&dir, word, &["install", "mine"]), 1);
    assert!(
        text.contains("exited with code 8: hinterland unreachable\n"),
        "{text}"
    );
}

#[test]
fn a_call_with_one_package_is_not_retried() {
    let dir = setup(
        "tool-single-bad",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"bad\"]\n",
    );
    write(&dir, "pyproject.toml", PYPROJECT);
    let script = format!("{UV_REJECTS_BAD}\n{UV_ADDS}");

    failed(&with_uv(&dir, &script, &["install", "mine"]), 1);

    assert_eq!(uv_calls(&dir), ["add --dev bad"]);
}

#[test]
fn the_reason_skips_the_hints_and_prefers_the_last_cause() {
    let dir = setup(
        "tool-reason",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\"]\n",
    );
    write(&dir, "pyproject.toml", PYPROJECT);
    let only_hints = "echo 'hint: one' >&2\necho 'hint: two' >&2\nexit 4";

    let text = failed(&with_uv(&dir, only_hints, &["install", "mine"]), 1);
    assert!(
        text.contains("tool ruff: failed: `uv add --dev` exited with code 4\n"),
        "{text}"
    );

    let errors = "echo 'cause: first' >&2\necho 'cause: second' >&2\necho 'last line' >&2\nexit 5";
    let text = failed(&with_uv(&dir, errors, &["install", "mine"]), 1);
    assert!(
        text.contains("exited with code 5: cause: second\n"),
        "{text}"
    );

    let plain = "echo 'first' >&2\necho 'last line' >&2\necho 'hint: x' >&2\nexit 6";
    let text = failed(&with_uv(&dir, plain, &["install", "mine"]), 1);
    assert!(text.contains("exited with code 6: last line\n"), "{text}");
}

#[test]
fn a_package_that_is_still_not_declared_after_the_call_fails() {
    let dir = setup("tool-absent", TOOLS);
    write(&dir, "pyproject.toml", PYPROJECT);

    let text = failed(&with_uv(&dir, "exit 0", &["install", "mine"]), 1);

    assert!(
        text.contains("tool ruff: failed: not in pyproject.toml after `uv add --dev`\n"),
        "{text}"
    );
}

#[test]
fn a_project_without_uv_fails_for_each_missing_package() {
    let dir = setup("tool-no-uv", TOOLS);
    write(&dir, "pyproject.toml", PYPROJECT);

    let text = failed(&without_npx(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("tool ruff: failed: cannot run `uv`: "),
        "{text}"
    );
    assert!(
        text.ends_with("mine: 0 added, 0 ok, 0 pending, 3 failed\n"),
        "{text}"
    );
}

#[test]
fn the_call_runs_in_the_root_of_the_project_after_the_files_and_before_the_skills() {
    let dir = setup(
        "tool-order",
        &format!(
            "{TOOLS}[[files]]\npath = \"pyproject.toml\"\ntemplate = \"files/pyproject.toml\"\n\
             [[skills]]\nsource = \"a/b\"\nnames = [\"x\"]\n"
        ),
    );
    write(&dir, "recipes/files/pyproject.toml", PYPROJECT);
    write(&dir, "deep/er/keep", "");

    let output = with_uv(&dir.join("deep/er"), UV_ADDS, &["install", "mine"]);

    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {text}"))
    };
    assert!(at("file pyproject.toml") < at("tool ruff"), "{text}");
    assert!(at("tool zensical") < at("skill x"), "{text}");
    assert!(dir.join(".uv-calls").is_file());
    assert!(!dir.join("deep/er/.uv-calls").exists());
}

#[test]
fn a_package_that_two_named_recipes_share_is_added_once() {
    let dir = setup(
        "tool-shared",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\"]\n",
    );
    add_recipe(
        &dir,
        "other",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\", \"pytest\"]\n",
    );
    write(&dir, "pyproject.toml", PYPROJECT);

    let text = ok(&with_uv(&dir, UV_ADDS, &["install", "mine", "other"]));

    assert_eq!(
        text,
        "tool ruff: added\ntool pytest: added\nmine, other: 2 added, 0 ok, 0 pending\n"
    );
    assert_eq!(uv_calls(&dir), ["add --dev ruff pytest"]);
}

#[test]
fn check_says_which_packages_pyproject_does_not_declare() {
    let dir = setup("tool-check", TOOLS);
    write(
        &dir,
        "pyproject.toml",
        "[dependency-groups]\ndev = [\"ruff\"]\n",
    );

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert_eq!(
        text,
        "tool ruff: ok\ntool pyrefly: differs: not in pyproject.toml\n\
         tool zensical: differs: not in pyproject.toml\nmine: 3 checked, 2 differ\n"
    );
}

#[test]
fn check_says_when_pyproject_is_missing_and_is_ok_after_install() {
    let dir = setup("tool-check-none", TOOLS);
    let text = failed(&recipe(&dir, &["check", "mine"]), 1);
    assert!(
        text.contains("tool ruff: differs: no pyproject.toml\n"),
        "{text}"
    );

    let dir = setup(
        "tool-check-ok",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\"]\n",
    );
    write(&dir, "pyproject.toml", PYPROJECT);
    ok(&with_uv(&dir, UV_ADDS, &["install", "mine"]));
    let text = ok(&recipe(&dir, &["check", "mine"]));
    assert_eq!(text, "tool ruff: ok\nmine: 1 checked, 0 differ\n");
}

#[test]
fn check_goes_through_the_packages_from_the_root_of_the_project() {
    let dir = setup(
        "tool-check-root",
        "[[tools]]\nmanager = \"uv\"\npackages = [\"ruff\"]\n",
    );
    write(
        &dir,
        "pyproject.toml",
        "[dependency-groups]\ndev = [\"ruff\"]\n",
    );
    write(&dir, "deep/er/keep", "");

    let text = ok(&recipe(&dir.join("deep/er"), &["check", "mine"]));

    assert!(text.ends_with("mine: 1 checked, 0 differ\n"), "{text}");
}
