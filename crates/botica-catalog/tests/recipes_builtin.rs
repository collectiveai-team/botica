//! The recipes that the binary carries.

use botica_catalog::{Catalog, Kind, Origin, Recipes};

fn embedded() -> Recipes {
    Recipes::embedded().expect("the built-in recipes load")
}

#[test]
fn the_binary_carries_its_recipes_with_a_description_and_the_origin_built_in() {
    let recipes = embedded();

    let ids: Vec<&str> = recipes.recipes().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids.len(), 32);
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
    assert_eq!(resolved.rules.len(), 33);
}

#[test]
fn no_utils_comes_from_base_for_every_language() {
    let recipes = embedded();
    let base = recipes.recipe("base").expect("base");
    let python = recipes.recipe("base-python").expect("base-python");

    assert!(base.rules.iter().any(|rule| rule == "no-utils"));
    assert!(!python.rules.iter().any(|rule| rule == "no-utils"));
    let resolved = recipes.resolve("base-python").expect("resolves");
    let no_utils = resolved
        .rules
        .iter()
        .find(|item| item.id == "no-utils")
        .expect("selected");
    assert_eq!(no_utils.from, "base");
}

#[test]
fn lockfile_committed_comes_from_base_and_no_other_recipe_lists_it() {
    let recipes = embedded();

    for recipe in recipes.recipes() {
        let lists = recipe.rules.iter().any(|rule| rule == "lockfile-committed");
        assert_eq!(lists, recipe.id == "base", "recipe {}", recipe.id);
    }
    let resolved = recipes.resolve("base-python").expect("resolves");
    let lockfile = resolved
        .rules
        .iter()
        .find(|item| item.id == "lockfile-committed")
        .expect("selected");
    assert_eq!(lockfile.from, "base");
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
    let files: Vec<String> = base
        .files
        .iter()
        .map(|f| format!("{} <- {}", f.path, f.template))
        .collect();
    assert_eq!(
        files,
        [
            "prek.toml <- files/prek-base.toml",
            ".github/workflows/abide.yml <- files/github/workflows/abide.yml",
            ".github/pull_request_template.md <- files/github/pull_request_template.md",
            ".github/ISSUE_TEMPLATE/bug_report.md <- files/github/ISSUE_TEMPLATE/bug_report.md",
            ".github/ISSUE_TEMPLATE/feature_proposal.md <- files/github/ISSUE_TEMPLATE/feature_proposal.md",
        ]
    );
    assert_eq!(base.sections.len(), 2);
    assert_eq!(base.sections[1].path, "AGENTS.md");
    assert_eq!(base.sections[1].marker, "## Botica rules");
    assert_eq!(base.sections[1].template, "sections/botica-rules.md");

    let prek = recipes
        .template("base", "files/prek-base.toml")
        .expect("the template is embedded");
    assert!(prek.starts_with("# MARK: uv-lock\n"), "{prek}");
    let index = recipes
        .template("base", "sections/botica-rules.md")
        .expect("the template is embedded");
    assert!(index.starts_with("## Botica rules\n"), "{index}");
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
    assert!(base.gitignore[0]
        .lines
        .contains(&"node_modules/".to_owned()));
    assert!(base.gitignore[0].lines.contains(&"target/".to_owned()));
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
            "pyproject.toml # MARK: uv",
            "pyproject.toml # MARK: Ruff",
            "pyproject.toml # MARK: Pyrefly",
            "pyproject.toml # MARK: Pytest",
        ]
    );
    assert_eq!(python.tools.len(), 1);
    assert_eq!(python.tools[0].manager, "uv");
    assert!(python.tools[0].packages.contains(&"ruff".to_owned()));
    let resolved = recipes.resolve("base-python").expect("resolves");
    assert!(resolved.skills.iter().all(|skill| skill.from == "base"));
    assert_eq!(resolved.files.len(), 6);
    assert_eq!(resolved.sections.len(), 9);
    assert!(resolved.warnings.is_empty());
}

#[test]
fn the_binary_carries_a_recipe_for_each_agent_and_each_piece_of_ci() {
    let recipes = embedded();
    let ids: Vec<&str> = recipes.recipes().iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "agent-claude-code",
            "agent-codex",
            "agent-opencode",
            "base",
            "base-go",
            "base-node",
            "base-python",
            "base-rust",
            "ci",
            "ci-automerge-private",
            "ci-automerge-public",
            "ci-dependency-review",
            "ci-docker",
            "ci-go",
            "ci-node",
            "ci-publish",
            "ci-python",
            "ci-rust",
            "database-alembic",
            "full-go",
            "full-node",
            "full-python",
            "full-rust",
            "iac-terraform",
            "python-strict-types",
            "python-test-smells",
            "repo-private",
            "repo-public",
            "secrets-varlock",
            "security",
            "testing-blockbuster",
            "testing-hypothesis"
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
fn secrets_varlock_installs_only_the_skill_of_varlock() {
    let recipes = embedded();

    let varlock = recipes.recipe("secrets-varlock").expect("secrets-varlock");
    let skills: Vec<String> = varlock
        .skills
        .iter()
        .map(|s| format!("{} {:?}", s.source, s.names))
        .collect();
    assert_eq!(skills, ["dmno-dev/varlock [\"varlock\"]"]);
    assert!(varlock.extends.is_empty() && varlock.agents.is_empty());
    assert!(varlock.files.is_empty() && varlock.sections.is_empty() && varlock.links.is_empty());
    assert!(varlock.gitignore.is_empty() && varlock.tools.is_empty() && varlock.rules.is_empty());
    assert!(varlock
        .description
        .as_deref()
        .is_some_and(|d| d.contains("varlock init --agent")));
}

#[test]
fn security_selects_the_rules_of_ces_121_and_has_no_step() {
    let recipes = embedded();
    let catalog = Catalog::embedded().expect("the catalog loads");

    let security = recipes.recipe("security").expect("security");
    assert_eq!(
        security.rules,
        ["route-auth", "jwt-verified", "no-token-in-web-storage"]
    );
    assert_eq!(security.judgment, ["short-lived-access-token"]);
    for id in security.rules.iter().chain(&security.judgment) {
        let rule = catalog.resolve(id).expect("in the catalog");
        assert_eq!(rule.standard.as_deref(), Some("CES-121"), "{id}");
    }
    assert!(security.extends.is_empty() && security.agents.is_empty());
    assert!(security.skills.is_empty() && security.files.is_empty());
    assert!(security.sections.is_empty() && security.links.is_empty());
    assert!(security.gitignore.is_empty() && security.tools.is_empty());
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
            "AGENTS.md ## Botica rules"
        ]
    );
}

fn paths(recipes: &Recipes, id: &str) -> Vec<String> {
    recipes
        .resolve(id)
        .expect("resolves")
        .files
        .iter()
        .filter(|file| file.path.starts_with(".github/"))
        .map(|file| file.path.clone())
        .collect()
}

#[test]
fn the_recipes_of_ci_have_the_workflows_one_piece_each() {
    let recipes = embedded();

    assert_eq!(
        paths(&recipes, "ci"),
        [
            ".github/workflows/conventional-commits.yml",
            ".github/workflows/commit-policy.yml",
            ".github/workflows/zizmor.yml",
            ".github/zizmor.yml"
        ]
    );
    assert_eq!(
        recipes.recipe("ci-python").expect("ci-python").extends,
        ["ci"]
    );
    assert_eq!(
        paths(&recipes, "ci-python")[4..],
        [
            ".github/workflows/tests.yml",
            ".github/dependabot.yml",
            ".github/workflows/osv-scanner.yml"
        ]
    );
    assert_eq!(
        paths(&recipes, "ci-dependency-review"),
        [".github/workflows/dependency-review.yml"]
    );
    assert_eq!(
        paths(&recipes, "ci-docker"),
        [".github/workflows/docker.yml"]
    );
    assert_eq!(
        paths(&recipes, "ci-publish"),
        [
            ".github/workflows/release.yml",
            ".github/workflows/pypi.yml"
        ]
    );
}

#[test]
fn a_public_repository_has_the_dependency_review_and_the_docker_workflows_and_a_private_one_does_not(
) {
    let recipes = embedded();

    let public = paths(&recipes, "repo-public");
    let private = paths(&recipes, "repo-private");

    assert_eq!(private, paths(&recipes, "ci"));
    assert_eq!(public.len(), 6);
    assert!(public.starts_with(&private));
    assert_eq!(
        public[4..],
        [
            ".github/workflows/dependency-review.yml",
            ".github/workflows/docker.yml"
        ]
    );
    assert_eq!(
        recipes
            .recipe("repo-public")
            .expect("repo-public")
            .extends
            .len(),
        3
    );
    assert_eq!(
        recipes
            .recipe("repo-private")
            .expect("repo-private")
            .extends,
        ["ci"]
    );
}

#[test]
fn testing_blockbuster_adds_blockbuster_bounded_on_the_minor_and_nothing_else() {
    let recipes = embedded();

    let blockbuster = recipes
        .recipe("testing-blockbuster")
        .expect("testing-blockbuster");
    let tools: Vec<String> = blockbuster
        .tools
        .iter()
        .map(|t| format!("{} {:?} {:?}", t.manager, t.group, t.packages))
        .collect();
    assert_eq!(tools, ["uv None [\"blockbuster>=1.5,<1.6\"]"]);
    assert!(blockbuster.extends.is_empty() && blockbuster.rules.is_empty());
    assert!(blockbuster.files.is_empty() && blockbuster.sections.is_empty());
    assert!(blockbuster.gitignore.is_empty() && blockbuster.skills.is_empty());
}

#[test]
fn testing_hypothesis_adds_only_hypothesis_and_its_folder_of_examples() {
    let recipes = embedded();

    let hypothesis = recipes
        .recipe("testing-hypothesis")
        .expect("testing-hypothesis");
    let tools: Vec<String> = hypothesis
        .tools
        .iter()
        .map(|t| format!("{} {:?} {:?}", t.manager, t.group, t.packages))
        .collect();
    assert_eq!(tools, ["uv None [\"hypothesis\"]"]);
    assert_eq!(hypothesis.gitignore.len(), 1);
    assert_eq!(hypothesis.gitignore[0].lines, [".hypothesis/"]);
    assert!(hypothesis.extends.is_empty() && hypothesis.rules.is_empty());
    assert!(hypothesis.files.is_empty() && hypothesis.sections.is_empty());
    assert!(hypothesis.skills.is_empty() && hypothesis.links.is_empty());
}
