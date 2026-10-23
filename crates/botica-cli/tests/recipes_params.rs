//! The parameters of a recipe: `install` writes their values into the templates, `--set` changes
//! them for one run and keeps them in `[recipes.params]`, and `show` lists them.

mod common;

use std::fs;
use std::path::Path;

use common::recipes::{add_recipe, project, recipe, template};
use common::refs::{failed, ok};

const MINE: &str = "[[files]]\npath = \"pnpm-workspace.yaml\"\ntemplate = \"files/pnpm.yaml\"\n\n\
                    [params.cooldown-days]\ndescription = \"Days a release waits\"\nkind = \"days\"\n\
                    default = 7\n";
const PNPM: &str = "minimumReleaseAge: {{ param.cooldown-days.minutes }}\n";

fn read(dir: &Path, path: &str) -> String {
    fs::read_to_string(dir.join(path)).expect("read the file")
}

fn mine(name: &str, config: Option<&str>) -> std::path::PathBuf {
    let dir = project(name, config);
    add_recipe(&dir, "mine", MINE);
    template(&dir, "files/pnpm.yaml", PNPM);
    dir
}

#[test]
fn install_writes_the_default_in_the_unit_of_the_template() {
    let dir = mine("params-default", None);

    ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(
        read(&dir, "pnpm-workspace.yaml"),
        "minimumReleaseAge: 10080\n"
    );
    assert!(!dir.join("botica.toml").exists());
}

#[test]
fn set_writes_its_value_and_keeps_it_in_a_new_botica_toml() {
    let dir = mine("params-set", None);

    let text = ok(&recipe(
        &dir,
        &["install", "mine", "--set", "cooldown-days=14"],
    ));

    assert!(text.contains("param cooldown-days: added\n"), "{text}");
    assert_eq!(
        read(&dir, "pnpm-workspace.yaml"),
        "minimumReleaseAge: 20160\n"
    );
    assert_eq!(
        read(&dir, "botica.toml"),
        "[recipes.params]\ncooldown-days = \"14\"\n"
    );
}

#[test]
fn the_configuration_gives_the_value_and_set_without_it_is_pending() {
    let dir = mine(
        "params-config",
        Some("[recipes.params]\ncooldown-days = 10\n"),
    );

    ok(&recipe(&dir, &["install", "mine"]));
    assert_eq!(
        read(&dir, "pnpm-workspace.yaml"),
        "minimumReleaseAge: 14400\n"
    );

    fs::remove_file(dir.join("pnpm-workspace.yaml")).expect("remove");
    let text = failed(
        &recipe(&dir, &["install", "mine", "--set", "cooldown-days=21"]),
        1,
    );
    assert!(
        text.contains(
            "param cooldown-days: pending: the configuration in `botica.toml` lacks \
             `[recipes.params] cooldown-days = \"21\"`"
        ),
        "{text}"
    );
    assert_eq!(
        read(&dir, "pnpm-workspace.yaml"),
        "minimumReleaseAge: 30240\n"
    );
}

#[test]
fn a_value_out_of_its_kind_and_a_name_no_recipe_declares_fail_before_writing() {
    let dir = mine("params-bad", None);

    let text = failed(
        &recipe(&dir, &["install", "mine", "--set", "cooldown-days=a week"]),
        2,
    );
    assert!(text.contains("is not a whole number of days"), "{text}");
    let text = failed(&recipe(&dir, &["install", "mine", "--set", "days=7"]), 2);
    assert!(text.contains("declares the parameter `days`"), "{text}");
    assert!(!dir.join("pnpm-workspace.yaml").exists());

    let dir = mine("params-bad-config", Some("[recipes.params]\ndays = 7\n"));
    let text = failed(&recipe(&dir, &["check", "mine"]), 2);
    assert!(
        text.contains("sets `days`, which no recipe declares"),
        "{text}"
    );
}

#[test]
fn a_template_that_writes_a_parameter_the_chain_lacks_does_not_resolve() {
    let dir = project("params-undeclared", None);
    add_recipe(
        &dir,
        "mine",
        "[[files]]\npath = \"x.yaml\"\ntemplate = \"files/x.yaml\"\n",
    );
    template(&dir, "files/x.yaml", "a: {{ param.cooldown-days }}\n");

    let text = failed(&recipe(&dir, &["install", "mine"]), 2);

    assert!(
        text.contains(
            "the template `files/x.yaml` of `mine`: it writes the parameter `cooldown-days`, \
             which no recipe of the chain declares"
        ),
        "{text}"
    );
}

#[test]
fn show_lists_the_parameters_with_their_value_and_list_counts_them() {
    let dir = mine(
        "params-show",
        Some("[recipes.params]\ncooldown-days = 14\n"),
    );

    let shown = ok(&recipe(&dir, &["show", "mine"]));
    let listed = ok(&recipe(&dir, &["list"]));

    assert!(
        shown.ends_with(
            "## Parameters\n\n- cooldown-days = \"14\" (days, default \"7\", from mine): \
             Days a release waits\n"
        ),
        "{shown}"
    );
    assert!(
        listed.contains("\nmine\n  rules: 0, judgment: 0, exports: 0\n  files: 1, params: 1\n"),
        "{listed}"
    );
}
