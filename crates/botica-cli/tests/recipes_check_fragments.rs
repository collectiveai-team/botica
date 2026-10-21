//! `botica recipe check` and the fragments of configuration of the external rules that a recipe
//! lists: the hooks of prek that `acme/complexity` needs, for one (ADR 0006).

mod common;

use common::external_rule::{
    complexity_rule, LACKS, SELECTS_COMPLEXITY, WITHOUT_THE_HOOK, WITH_THE_HOOK,
};
use common::recipes::{add_recipe, project, recipe};
use common::refs::{failed, ok};
use common::write;

fn tooling(name: &str, config: &str, prek: Option<&str>) -> std::path::PathBuf {
    let dir = project(name, Some(config));
    complexity_rule(&dir);
    add_recipe(&dir, "tooling", "rules = [\"acme/complexity\"]\n");
    if let Some(text) = prek {
        write(&dir, "prek.toml", text);
    }
    dir
}

#[test]
fn a_file_that_holds_the_fragment_of_an_external_rule_of_the_recipe_is_ok() {
    let dir = tooling("frag-ok", SELECTS_COMPLEXITY, Some(WITH_THE_HOOK));

    let text = ok(&recipe(&dir, &["check", "tooling"]));

    assert_eq!(
        text,
        "rule acme/complexity: ok\n\
         fragment prek.toml `acme/complexity`: ok\n\
         tooling: 2 checked, 0 differ\n"
    );
}

#[test]
fn a_file_that_lacks_the_fragment_differs() {
    let dir = tooling("frag-lacks", SELECTS_COMPLEXITY, Some(WITHOUT_THE_HOOK));

    let text = failed(&recipe(&dir, &["check", "tooling"]), 1);

    assert_eq!(
        text,
        format!(
            "rule acme/complexity: ok\n\
             fragment prek.toml `acme/complexity`: differs: {LACKS}\n\
             tooling: 2 checked, 1 differs\n"
        )
    );
}

#[test]
fn a_file_that_does_not_exist_differs() {
    let dir = tooling("frag-missing", SELECTS_COMPLEXITY, None);

    let text = failed(&recipe(&dir, &["check", "tooling"]), 1);

    assert!(
        text.contains(
            "fragment prek.toml `acme/complexity`: differs: prek.toml does not exist, and \
             complexipy needs it for `acme/complexity`"
        ),
        "{text}"
    );
}

#[test]
fn a_file_that_is_not_toml_differs() {
    let dir = tooling("frag-not-toml", SELECTS_COMPLEXITY, Some("[[repos\n"));

    let text = failed(&recipe(&dir, &["check", "tooling"]), 1);

    assert!(
        text.contains("fragment prek.toml `acme/complexity`: differs: prek.toml is not TOML"),
        "{text}"
    );
}

#[test]
fn a_rule_that_the_project_does_not_run_differs_and_its_fragment_is_still_checked() {
    let dir = tooling("frag-inactive", "preview = true\n", Some(WITH_THE_HOOK));

    let text = failed(&recipe(&dir, &["check", "tooling"]), 1);

    assert!(text.contains("rule acme/complexity: differs:"), "{text}");
    assert!(
        text.contains("fragment prek.toml `acme/complexity`: ok"),
        "{text}"
    );
    assert!(text.ends_with("tooling: 2 checked, 1 differs\n"), "{text}");
}

#[test]
fn a_rule_that_is_not_in_the_catalog_has_no_fragment_line() {
    let dir = project("frag-unknown", None);
    add_recipe(&dir, "tooling", "rules = [\"no-such-rule\"]\n");

    let text = failed(&recipe(&dir, &["check", "tooling"]), 1);

    assert_eq!(
        text,
        "rule no-such-rule: differs: not in the catalog\ntooling: 1 checked, 1 differs\n"
    );
}

#[test]
fn the_fragments_come_after_the_rules_and_before_the_files() {
    let dir = project("frag-order", Some(SELECTS_COMPLEXITY));
    complexity_rule(&dir);
    add_recipe(
        &dir,
        "tooling",
        "rules = [\"acme/complexity\"]\n\n[[files]]\npath = \"a.txt\"\ntemplate = \"files/a.txt\"\n",
    );
    write(&dir, "prek.toml", WITH_THE_HOOK);
    write(&dir, "a.txt", "x\n");

    let text = ok(&recipe(&dir, &["check", "tooling"]));

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[..3],
        [
            "rule acme/complexity: ok",
            "fragment prek.toml `acme/complexity`: ok",
            "file a.txt: ok"
        ]
    );
}
