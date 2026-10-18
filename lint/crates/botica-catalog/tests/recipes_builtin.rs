//! The recipes that the binary carries.

use botica_catalog::{Catalog, Kind, Origin, Recipes};

fn embedded() -> Recipes {
    Recipes::embedded().expect("the built-in recipes load")
}

#[test]
fn the_binary_carries_its_recipes_with_a_description_and_the_origin_built_in() {
    let recipes = embedded();

    let ids: Vec<&str> = recipes.recipes().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids.len(), 5);
    for recipe in recipes.recipes() {
        assert_eq!(recipe.origin, Origin::BuiltIn);
        assert!(
            recipe.description.is_some(),
            "{} has no description",
            recipe.id
        );
    }
    assert_eq!(
        recipes.recipe("base-python").expect("base-python").extends,
        ["base"]
    );
}

#[test]
fn every_rule_of_a_built_in_recipe_is_in_the_built_in_catalog() {
    let recipes = embedded();
    let catalog = Catalog::embedded().expect("the catalog loads");

    for recipe in recipes.recipes() {
        for id in recipe.rules.iter().chain(&recipe.judgment) {
            assert!(
                catalog.resolve(id).is_some(),
                "recipe {} lists `{id}`, which is not in the catalog",
                recipe.id
            );
        }
    }
}

#[test]
fn a_built_in_recipe_lists_judgment_rules_apart_from_the_others() {
    let recipes = embedded();
    let catalog = Catalog::embedded().expect("the catalog loads");
    let is_judgment = |id: &String| {
        catalog.resolve(id).is_some_and(|rule| {
            rule.implementations
                .iter()
                .any(|i| i.kind == Kind::Judgment)
        })
    };

    for recipe in recipes.recipes() {
        assert!(
            !recipe.rules.iter().any(is_judgment),
            "recipe {} lists a judgment rule under `rules`",
            recipe.id
        );
        assert!(
            recipe.judgment.iter().all(is_judgment),
            "recipe {} lists under `judgment` a rule that is not a judgment rule",
            recipe.id
        );
    }
}

#[test]
fn base_python_resolves_to_the_rules_of_base_first() {
    let resolved = embedded().resolve("base-python").expect("resolves");

    assert_eq!(resolved.chain, ["base", "base-python"]);
    let first: Vec<&str> = resolved
        .rules
        .iter()
        .take(3)
        .map(|i| i.id.as_str())
        .collect();
    assert_eq!(
        first,
        ["file-size-guard", "no-notebooks", "no-artifacts-in-package"]
    );
    assert_eq!(resolved.rules.len(), 15);
}

#[test]
fn base_installs_the_skills_and_adds_prek_and_the_standards_index_from_its_templates() {
    let recipes = embedded();
    let base = recipes.recipe("base").expect("base");

    let sources: Vec<&str> = base.skills.iter().map(|s| s.source.as_str()).collect();
    assert!(sources[0].starts_with("mattpocock/skills#v"), "{sources:?}");
    assert_eq!(sources[1], "collectiveai-team/botica");
    assert_eq!(sources.len(), 2);
    assert!(base.skills.iter().all(|step| !step.names.is_empty()));
    assert_eq!(base.files.len(), 1);
    assert_eq!(base.files[0].path, "prek.toml");
    assert_eq!(base.files[0].template, "files/prek-base.toml");
    assert_eq!(base.sections.len(), 2);
    assert_eq!(base.sections[1].path, "AGENTS.md");
    assert_eq!(base.sections[1].marker, "## Engineering Standards");
    assert_eq!(base.sections[1].template, "sections/standards-index.md");

    let prek = recipes
        .template("base", "files/prek-base.toml")
        .expect("the template is embedded");
    assert!(prek.starts_with("# MARK: uv-lock\n"), "{prek}");
    let index = recipes
        .template("base", "sections/standards-index.md")
        .expect("the template is embedded");
    assert!(index.starts_with("## Engineering Standards\n"), "{index}");
}

#[test]
fn base_ignores_the_workspace_folders_and_lets_the_manifest_of_skills_out() {
    let recipes = embedded();
    let base = recipes.recipe("base").expect("base");

    assert_eq!(base.gitignore.len(), 1);
    assert!(base.gitignore[0]
        .lines
        .contains(&".agents/skills/".to_owned()));
    assert!(base.gitignore[0].lines.contains(&".tmp/".to_owned()));
    assert_eq!(base.gitignore[0].unignore, ["skills-lock.json"]);
}

#[test]
fn base_python_adds_pyproject_the_python_hooks_and_the_dev_tools_to_what_base_does() {
    let recipes = embedded();

    let python = recipes.recipe("base-python").expect("base-python");
    assert!(python.skills.is_empty() && python.links.is_empty() && python.gitignore.is_empty());
    assert_eq!(python.files.len(), 1);
    assert_eq!(python.files[0].path, "pyproject.toml");
    let sections: Vec<String> = python
        .sections
        .iter()
        .map(|s| format!("{} {}", s.path, s.marker))
        .collect();
    assert_eq!(
        sections,
        [
            "prek.toml # MARK: validate-pyproject",
            "prek.toml # MARK: ruff-format",
            "prek.toml # MARK: pyrefly",
            "pyproject.toml # MARK: Ruff",
            "pyproject.toml # MARK: Pyrefly",
            "pyproject.toml # MARK: import-linter",
            "pyproject.toml # MARK: Pytest",
        ]
    );
    assert_eq!(python.tools.len(), 1);
    assert_eq!(python.tools[0].manager, "uv");
    assert!(python.tools[0].packages.contains(&"ruff".to_owned()));
    let resolved = recipes.resolve("base-python").expect("resolves");
    assert!(resolved.skills.iter().all(|skill| skill.from == "base"));
    assert_eq!(resolved.files.len(), 2);
    assert_eq!(resolved.sections.len(), 9);
    assert!(resolved.warnings.is_empty());
}

#[test]
fn there_is_a_recipe_for_each_agent() {
    let recipes = embedded();
    let ids: Vec<&str> = recipes.recipes().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "agent-claude-code",
            "agent-codex",
            "agent-opencode",
            "base",
            "base-python"
        ]
    );

    let opencode = recipes.recipe("agent-opencode").expect("agent-opencode");
    assert_eq!(opencode.agents, ["opencode"]);
    assert_eq!(opencode.files.len(), 1);
    assert_eq!(opencode.files[0].path, "opencode.jsonc");
    assert!(opencode.links.is_empty());

    let claude = recipes
        .recipe("agent-claude-code")
        .expect("agent-claude-code");
    assert!(claude.agents.is_empty());
    assert_eq!(claude.files.len(), 1);
    assert_eq!(claude.files[0].path, ".claude/settings.json");
    let links: Vec<String> = claude
        .links
        .iter()
        .map(|l| format!("{} -> {}", l.path, l.target))
        .collect();
    assert_eq!(
        links,
        [
            "CLAUDE.md -> AGENTS.md",
            ".claude/skills -> ../.agents/skills"
        ]
    );

    let codex = recipes.recipe("agent-codex").expect("agent-codex");
    assert_eq!(codex.agents, ["codex"]);
    assert!(codex.files.is_empty() && codex.links.is_empty() && codex.sections.is_empty());
}

#[test]
fn base_adds_the_workspace_defaults_to_agents_md_before_the_standards_index() {
    let recipes = embedded();
    let base = recipes.recipe("base").expect("base");

    let sections: Vec<String> = base
        .sections
        .iter()
        .map(|s| format!("{} {}", s.path, s.marker))
        .collect();

    assert_eq!(
        sections,
        [
            "AGENTS.md ## Repo Workspace Defaults",
            "AGENTS.md ## Engineering Standards"
        ]
    );
}
