//! `botica recipe check` with more than one name: the recipes are one, as for `install`.

mod common;

use common::recipes::{add_recipe, project, recipe};
use common::refs::{failed, ok};
use common::write;

const BASE: &str = "[[files]]\npath = \"shared.txt\"\ntemplate = \"files/shared.txt\"\n";

fn two_recipes(name: &str) -> std::path::PathBuf {
    let dir = project(name, None);
    add_recipe(&dir, "shared", BASE);
    add_recipe(
        &dir,
        "first",
        "extends = [\"shared\"]\n\n[[files]]\npath = \"one.txt\"\ntemplate = \"files/one.txt\"\n",
    );
    add_recipe(
        &dir,
        "second",
        "extends = [\"shared\"]\n\n[[files]]\npath = \"two.txt\"\ntemplate = \"files/two.txt\"\n",
    );
    dir
}

#[test]
fn the_steps_of_all_the_names_are_reported_each_once_under_one_summary() {
    let dir = two_recipes("many-ok");
    for file in ["shared.txt", "one.txt", "two.txt"] {
        write(&dir, file, "x\n");
    }

    let text = ok(&recipe(&dir, &["check", "first", "second"]));

    assert_eq!(
        text,
        "file shared.txt: ok\nfile one.txt: ok\nfile two.txt: ok\n\
         first, second: 3 checked, 0 differ\n"
    );
}

#[test]
fn a_difference_in_any_of_the_recipes_makes_the_command_fail() {
    let dir = two_recipes("many-differs");
    write(&dir, "shared.txt", "x\n");
    write(&dir, "one.txt", "x\n");

    let text = failed(&recipe(&dir, &["check", "first", "second"]), 1);

    assert!(text.contains("file two.txt: differs:"), "{text}");
    assert!(
        text.ends_with("first, second: 3 checked, 1 differs\n"),
        "{text}"
    );
}

#[test]
fn a_name_that_is_not_a_recipe_stops_the_command() {
    let dir = two_recipes("many-unknown");

    let text = failed(&recipe(&dir, &["check", "first", "nope"]), 2);

    assert!(text.contains("no recipe `nope`"), "{text}");
}

#[test]
fn a_skill_that_a_later_name_takes_from_another_source_is_a_warning() {
    let dir = project("many-skill", None);
    add_recipe(
        &dir,
        "a",
        "[[skills]]\nsource = \"x/one\"\nnames = [\"tdd\"]\n",
    );
    add_recipe(
        &dir,
        "b",
        "[[skills]]\nsource = \"y/two\"\nnames = [\"tdd\"]\n",
    );

    let text = failed(&recipe(&dir, &["check", "a", "b"]), 1);

    assert!(
        text.contains(
            "warning: the skill `tdd` comes from `y/two` (b), which replaces `x/one` (a)"
        ),
        "{text}"
    );
}

#[test]
fn one_name_is_still_enough() {
    let dir = two_recipes("many-one");
    write(&dir, "shared.txt", "x\n");

    let text = ok(&recipe(&dir, &["check", "shared"]));

    assert_eq!(text, "file shared.txt: ok\nshared: 1 checked, 0 differ\n");
}
