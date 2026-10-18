//! `botica recipe install` installs the skills of a recipe with `npx skills add` for the names
//! that are not in `.agents/skills/`. The tests put a `npx` of their own first on the `PATH`.
#![cfg(unix)]

mod common;

use common::recipes::{
    add_recipe, install_skill, npx_calls, project, template, with_npx, without_npx, NPX_INSTALLS,
};
use common::refs::{failed, ok};
use common::write;

const SKILLS: &str = "[[skills]]\nsource = \"acme/skills\"\nnames = [\"tdd\", \"grilling\"]\n";
const FILE: &str = "[[files]]\npath = \"prek.toml\"\ntemplate = \"files/prek.toml\"\n";
const SECTION: &str = "[[sections]]\npath = \"AGENTS.md\"\nmarker = \"## Standards\"\n\
                       template = \"sections/standards.md\"\n";
const INSTALLS_NOTHING: &str = "exit 0";

fn step(source: &str, names: &str) -> String {
    format!("[[skills]]\nsource = \"{source}\"\nnames = [{names}]\n")
}

#[test]
fn the_missing_skills_of_a_source_are_installed_with_one_call() {
    let dir = project("skills-add", None);
    add_recipe(&dir, "mine", SKILLS);

    let text = ok(&with_npx(&dir, NPX_INSTALLS, &["install", "mine"]));

    assert_eq!(
        text,
        "skill tdd: added\nskill grilling: added\nmine: 2 added, 0 ok, 0 pending\n"
    );
    assert_eq!(
        npx_calls(&dir),
        ["skills add acme/skills --agent opencode --yes --skill tdd grilling"]
    );
    assert!(dir.join(".agents/skills/tdd/SKILL.md").is_file());
}

#[test]
fn a_skill_that_is_installed_is_ok_and_npx_is_not_called() {
    let dir = project("skills-ok", None);
    add_recipe(&dir, "mine", SKILLS);
    install_skill(&dir, "tdd");
    install_skill(&dir, "grilling");

    let text = ok(&without_npx(&dir, &["install", "mine"]));

    assert_eq!(
        text,
        "skill tdd: ok\nskill grilling: ok\nmine: 0 added, 2 ok, 0 pending\n"
    );
}

#[test]
fn only_the_names_that_are_missing_are_asked_for() {
    let dir = project("skills-some", None);
    add_recipe(&dir, "mine", SKILLS);
    install_skill(&dir, "tdd");

    let text = ok(&with_npx(&dir, NPX_INSTALLS, &["install", "mine"]));

    assert!(
        text.starts_with("skill tdd: ok\nskill grilling: added\n"),
        "{text}"
    );
    assert_eq!(
        npx_calls(&dir),
        ["skills add acme/skills --agent opencode --yes --skill grilling"]
    );
}

#[test]
fn a_folder_without_the_skill_file_does_not_count_as_installed() {
    let dir = project("skills-empty-dir", None);
    add_recipe(&dir, "mine", &step("acme/skills", "\"tdd\""));
    write(&dir, ".agents/skills/tdd/notes.md", "");

    ok(&with_npx(&dir, NPX_INSTALLS, &["install", "mine"]));

    assert_eq!(npx_calls(&dir).len(), 1);
}

#[test]
fn there_is_one_call_for_each_source_in_the_order_they_first_appear() {
    let dir = project("skills-sources", None);
    let toml = format!(
        "{}{}{}",
        step("a/one", "\"x\""),
        step("b/two", "\"y\""),
        step("a/one", "\"z\"")
    );
    add_recipe(&dir, "mine", &toml);

    let text = ok(&with_npx(&dir, NPX_INSTALLS, &["install", "mine"]));

    assert!(
        text.starts_with("skill x: added\nskill y: added\nskill z: added\n"),
        "{text}"
    );
    assert_eq!(
        npx_calls(&dir),
        [
            "skills add a/one --agent opencode --yes --skill x z",
            "skills add b/two --agent opencode --yes --skill y"
        ]
    );
}

#[test]
fn a_call_that_fails_is_reported_and_the_other_sources_still_run() {
    let dir = project("skills-fails", None);
    add_recipe(
        &dir,
        "mine",
        &format!("{}{}", step("a/one", "\"x\""), step("b/two", "\"y\"")),
    );
    let script = format!(
        "case \"$*\" in *a/one*)\n  echo \"$*\" >> \"$PWD/.npx-calls\"\n  echo noise >&2\n  \
         echo '  boom  ' >&2\n  exit 1;;\nesac\n{NPX_INSTALLS}"
    );

    let text = failed(&with_npx(&dir, &script, &["install", "mine"]), 1);

    assert!(
        text.contains("skill x: failed: `npx skills add a/one` exited with code 1: boom\n"),
        "{text}"
    );
    assert!(text.contains("skill y: added\n"), "{text}");
    assert!(
        text.ends_with("mine: 1 added, 0 ok, 0 pending, 1 failed\n"),
        "{text}"
    );
    assert_eq!(npx_calls(&dir).len(), 2);
}

#[test]
fn a_call_that_fails_without_a_message_or_is_stopped_says_how() {
    let dir = project("skills-no-message", None);
    add_recipe(&dir, "mine", &step("a/one", "\"x\""));

    let text = failed(&with_npx(&dir, "exit 3", &["install", "mine"]), 1);
    assert!(
        text.contains("skill x: failed: `npx skills add a/one` exited with code 3\n"),
        "{text}"
    );

    let text = failed(&with_npx(&dir, "kill -9 $$", &["install", "mine"]), 1);
    assert!(
        text.contains("skill x: failed: `npx skills add a/one` was stopped by a signal\n"),
        "{text}"
    );
}

#[test]
fn the_call_runs_in_the_root_of_the_project() {
    let dir = project("skills-root", None);
    add_recipe(&dir, "mine", &step("a/one", "\"x\""));
    write(&dir, "deep/er/keep", "");

    ok(&with_npx(
        &dir.join("deep/er"),
        NPX_INSTALLS,
        &["install", "mine"],
    ));

    assert!(dir.join(".agents/skills/x/SKILL.md").is_file());
    assert!(!dir.join("deep/er/.agents").exists());
}

#[test]
fn a_skill_that_is_still_missing_after_the_call_fails_the_command() {
    let dir = project("skills-absent", None);
    add_recipe(&dir, "mine", SKILLS);

    let text = failed(&with_npx(&dir, INSTALLS_NOTHING, &["install", "mine"]), 1);

    assert!(
        text.contains(
            "skill tdd: failed: not in `.agents/skills/` after `npx skills add acme/skills`\n"
        ),
        "{text}"
    );
    assert!(
        text.ends_with("mine: 0 added, 0 ok, 0 pending, 2 failed\n"),
        "{text}"
    );
}

#[test]
fn a_skill_that_the_call_leaves_out_fails_and_the_others_are_added() {
    let dir = project("skills-partial", None);
    add_recipe(&dir, "mine", SKILLS);
    let script = "mkdir -p .agents/skills/tdd\necho s > .agents/skills/tdd/SKILL.md";

    let text = failed(&with_npx(&dir, script, &["install", "mine"]), 1);

    assert!(
        text.starts_with("skill tdd: added\nskill grilling: failed: not in"),
        "{text}"
    );
    assert!(
        text.ends_with("mine: 1 added, 0 ok, 0 pending, 1 failed\n"),
        "{text}"
    );
}

#[test]
fn a_project_without_npx_fails_for_each_missing_skill() {
    let dir = project("skills-no-npx", None);
    add_recipe(&dir, "mine", SKILLS);

    let text = failed(&without_npx(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("skill tdd: failed: cannot run `npx`: "),
        "{text}"
    );
    assert!(
        text.contains("skill grilling: failed: cannot run `npx`: "),
        "{text}"
    );
    assert!(
        text.ends_with("mine: 0 added, 0 ok, 0 pending, 2 failed\n"),
        "{text}"
    );
}

#[test]
fn the_skills_come_after_the_files_and_the_sections_and_those_are_written_when_a_call_fails() {
    let dir = project("skills-order", None);
    add_recipe(&dir, "mine", &format!("{SKILLS}{SECTION}{FILE}"));
    template(&dir, "files/prek.toml", "repos = []\n");
    template(&dir, "sections/standards.md", "## Standards\n");

    let text = failed(&with_npx(&dir, INSTALLS_NOTHING, &["install", "mine"]), 1);

    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {text}"))
    };
    assert!(at("file prek.toml") < at("section AGENTS.md"), "{text}");
    assert!(at("section AGENTS.md") < at("skill tdd"), "{text}");
    assert!(dir.join("prek.toml").is_file());
    assert!(dir.join("AGENTS.md").is_file());
}

#[test]
fn a_skill_that_two_recipes_share_is_installed_once() {
    let dir = project("skills-shared", None);
    add_recipe(&dir, "parent", &step("acme/skills", "\"tdd\""));
    add_recipe(&dir, "child", "extends = [\"parent\"]\n");

    let output = with_npx(&dir, NPX_INSTALLS, &["install", "parent", "child"]);

    let text = ok(&output);
    assert_eq!(
        text,
        "skill tdd: added\nparent, child: 1 added, 0 ok, 0 pending\n"
    );
    assert_eq!(npx_calls(&dir).len(), 1);
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn a_skill_that_the_later_recipe_takes_from_another_source_is_installed_from_it_with_a_warning() {
    let dir = project("skills-replaced", None);
    add_recipe(&dir, "first", &step("x/one", "\"tdd\""));
    add_recipe(&dir, "second", &step("y/two", "\"tdd\""));

    let output = with_npx(&dir, NPX_INSTALLS, &["install", "first", "second"]);

    let text = ok(&output);
    assert!(
        text.starts_with("skill tdd: added\nfirst, second: 1 added"),
        "{text}"
    );
    assert_eq!(
        npx_calls(&dir),
        ["skills add y/two --agent opencode --yes --skill tdd"]
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "warning: the skill `tdd` comes from `y/two` (second), which replaces `x/one` (first)"
        ),
        "{stderr}"
    );
}

#[test]
fn the_warning_of_a_recipe_that_replaces_a_skill_of_what_it_extends_goes_to_stderr() {
    let dir = project("skills-warning", None);
    add_recipe(&dir, "parent", &step("a/one", "\"tdd\""));
    add_recipe(
        &dir,
        "child",
        &format!("extends = [\"parent\"]\n{}", step("b/two", "\"tdd\"")),
    );

    let output = with_npx(&dir, NPX_INSTALLS, &["install", "child"]);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("warning: the skill `tdd` comes from `b/two`"),
        "{stderr}"
    );
    assert_eq!(npx_calls(&dir).len(), 1);
}

#[test]
fn the_lock_of_skills_is_left_to_the_cli_of_skills() {
    let dir = project("skills-lock", None);
    add_recipe(&dir, "mine", SKILLS);

    ok(&with_npx(&dir, NPX_INSTALLS, &["install", "mine"]));

    assert!(!dir.join("skills-lock.json").exists());
}
