//! `botica recipe list` and `botica recipe show` read the recipes of the binary and of the project.

mod common;

use common::recipes::{add_recipe, project, recipe};
use common::refs::{failed, ok};

#[test]
fn list_prints_the_built_in_recipes_with_what_they_extend_and_how_much_they_list() {
    let dir = project("list-built-in", None);

    let text = ok(&recipe(&dir, &["list"]));

    assert!(
        text.contains(
            "\nbase: Any repository: the rules about any file, the skills, .gitignore, the prek hooks \
             and the sections of AGENTS.md\n  rules: 3, judgment: 0, exports: 0\n  \
             skills: 32, files: 1, sections: 2, gitignore: 8\n  from: built-in\n"
        ),
        "{text}"
    );
    assert!(text.contains("\nbase-python: A Python project"), "{text}");
    assert!(
        text.contains(
            "  extends: base\n  rules: 12, judgment: 0, exports: 0\n  \
             files: 1, sections: 7, tools: 6\n  from: built-in\n"
        ),
        "{text}"
    );
}

#[test]
fn list_adds_the_recipes_of_the_project_in_the_order_of_their_ids() {
    let dir = project("list-project", None);
    add_recipe(
        &dir,
        "mine",
        "description = \"Ours\"\nextends = [\"base\"]\nrules = [\"no-utils\"]\n\
         judgment = [\"a\", \"b\"]\nexports = [\"abide\"]\n",
    );

    let text = ok(&recipe(&dir, &["list"]));

    let ids: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with(' '))
        .map(|line| line.split(':').next().unwrap_or_default())
        .collect();
    assert_eq!(
        ids,
        [
            "agent-claude-code",
            "agent-codex",
            "agent-opencode",
            "base",
            "base-python",
            "ci",
            "ci-automerge-private",
            "ci-automerge-public",
            "ci-dependency-review",
            "ci-docker",
            "ci-go",
            "ci-node",
            "ci-publish",
            "ci-python",
            "mine",
            "repo-private",
            "repo-public",
            "secrets-varlock"
        ],
        "{text}"
    );
    assert!(
        text.contains(
            "mine: Ours\n  extends: base\n  rules: 1, judgment: 2, exports: 1\n  from: project\n"
        ),
        "{text}"
    );
}

#[test]
fn a_recipe_without_a_description_is_listed_by_its_id() {
    let dir = project("list-bare", None);
    add_recipe(&dir, "bare", "");

    let text = ok(&recipe(&dir, &["list"]));

    assert!(
        text.contains("\nbare\n  rules: 0, judgment: 0, exports: 0\n  from: project\n"),
        "{text}"
    );
}

#[test]
fn a_recipe_of_the_project_with_a_built_in_id_fails_unless_the_configuration_replaces_it() {
    let dir = project("list-collision", None);
    add_recipe(&dir, "base", "rules = [\"file-size-guard\"]\n");

    let text = failed(&recipe(&dir, &["list"]), 2);
    assert!(text.contains("base.toml"), "{text}");
    assert!(text.contains("[recipes] replace"), "{text}");

    let dir = project("list-replace", Some("[recipes]\nreplace = [\"base\"]\n"));
    add_recipe(&dir, "base", "rules = [\"file-size-guard\"]\n");
    let text = ok(&recipe(&dir, &["list"]));
    assert!(
        text.contains("\nbase\n  rules: 1, judgment: 0, exports: 0\n  from: project\n"),
        "{text}"
    );
}

#[test]
fn a_recipe_file_that_does_not_load_fails_and_names_it() {
    let dir = project("list-bad", None);
    add_recipe(&dir, "broken", "exports = [\"pdf\"]\n");

    let text = failed(&recipe(&dir, &["list"]), 2);

    assert!(text.contains("recipes/ does not load"), "{text}");
    assert!(text.contains("broken.toml"), "{text}");
    assert!(text.contains("unknown export `pdf`"), "{text}");
}

#[test]
fn show_prints_the_rules_of_a_recipe_after_the_ones_it_extends_with_their_code_and_origin() {
    let dir = project("show-base-python", None);

    let text = ok(&recipe(&dir, &["show", "base-python"]));

    assert!(
        text.starts_with("# base-python: A Python project"),
        "{text}"
    );
    assert!(text.contains("\nExtends: base\n"), "{text}");
    let rules = text.split("## Rules\n").nth(1).expect("a section of rules");
    let lines: Vec<&str> = rules
        .split("\n## ")
        .next()
        .expect("the section")
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect();
    assert_eq!(lines.len(), 15, "{text}");
    assert!(
        lines[0].starts_with("- file-size-guard (BT"),
        "{}",
        lines[0]
    );
    assert!(lines[0].contains(", from base): "), "{}", lines[0]);
    assert!(lines[3].starts_with("- no-utils (BT"), "{}", lines[3]);
    assert!(lines[3].contains(", from base-python): "), "{}", lines[3]);
    assert!(text.contains("\n## Judgment rules\n\n(none)\n"), "{text}");
    assert!(text.contains("\n## Exports\n\n(none)\n"), "{text}");
    assert!(
        text.contains("\n## Exports\n\n(none)\n\n## Skills\n\n- grill-with-docs ("),
        "{text}"
    );
    assert!(
        text.contains(
            "\n## Files\n\n- prek.toml (template files/prek-base.toml, from base)\n\
             - pyproject.toml (template files/pyproject-base.toml, from base-python)\n\n\
             ## Sections\n\n- AGENTS.md, `## Repo Workspace Defaults` \
             (template sections/agents-workspace-defaults.md, from base)\n\
             - AGENTS.md, `## Engineering Standards` \
             (template sections/standards-index.md, from base)\n\
             - prek.toml, `# MARK: validate-pyproject` \
             (template sections/prek-validate-pyproject.toml, from base-python)\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("\n## Gitignore\n\n- .env (from base)\n- !.env.schema (from base)\n"),
        "{text}"
    );
    assert!(
        text.ends_with(
            "\n## Unignore\n\n- skills-lock.json (from base)\n\n## Tools\n\n\
             - ruff (uv, group dev, from base-python)\n\
             - pyrefly (uv, group dev, from base-python)\n\
             - pytest (uv, group dev, from base-python)\n\
             - pytest-asyncio (uv, group dev, from base-python)\n\
             - pytest-timeout (uv, group dev, from base-python)\n\
             - pytest-randomly (uv, group dev, from base-python)\n"
        ),
        "{text}"
    );
}

#[test]
fn show_names_the_exports_and_the_rules_that_the_catalog_does_not_have() {
    let dir = project("show-mine", None);
    add_recipe(
        &dir,
        "mine",
        "extends = [\"base\"]\nrules = [\"ghost\"]\njudgment = [\"phantom\"]\n\
         exports = [\"agents-md\"]\n",
    );

    let text = ok(&recipe(&dir, &["show", "mine"]));

    assert!(text.starts_with("# mine\n"), "{text}");
    assert!(
        text.contains("- ghost (not in the catalog, from mine)\n"),
        "{text}"
    );
    assert!(
        text.contains("## Judgment rules\n\n- phantom (not in the catalog, from mine)\n"),
        "{text}"
    );
    assert!(
        text.contains("## Exports\n\n- agents-md (from mine)\n"),
        "{text}"
    );
}

#[test]
fn show_a_recipe_that_does_not_exist_is_an_error() {
    let dir = project("show-unknown", None);

    let text = failed(&recipe(&dir, &["show", "nope"]), 2);

    assert!(text.contains("no recipe `nope`"), "{text}");
}
