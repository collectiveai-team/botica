//! `base-node`: the rules about TypeScript and JavaScript code, the cooldown of pnpm, Biome, ESLint
//! for JSDoc, tsc and knip from the packages of the project, the `tools` step that adds those
//! packages, and `full-node`.

mod common;

use botica_catalog::Recipes;
use common::workflows::{parse, text};

fn recipes() -> Recipes {
    Recipes::embedded().expect("the built-in recipes load")
}

/// The marker and the template of each section of `prek.toml`.
fn sections() -> Vec<(String, String)> {
    let recipes = recipes();
    let found = recipes.recipe("base-node").expect("base-node");
    found
        .sections
        .iter()
        .filter(|s| s.path == "prek.toml")
        .map(|s| (s.marker.clone(), s.template.clone()))
        .collect()
}

#[test]
fn base_node_extends_base_and_selects_the_rules_that_read_typescript() {
    let recipes = recipes();

    let node = recipes.recipe("base-node").expect("base-node");

    assert_eq!(node.extends, ["base"]);
    assert_eq!(
        node.rules,
        [
            "no-utils",
            "log-no-print",
            "no-disabled-typecheck",
            "cognitive-complexity",
            "lockfile-committed",
            "docstring-matches-signature",
            "pnpm-cooldown",
            "npm-cooldown",
            "yarn-cooldown"
        ]
    );
    let paths: Vec<&str> = node.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["biome.json", "eslint.config.js", "knip.json"]);
    assert!(node.gitignore.is_empty());
    let markers: Vec<String> = sections().into_iter().map(|s| s.0).collect();
    assert_eq!(
        markers,
        [
            "# MARK: biome",
            "# MARK: eslint-jsdoc",
            "# MARK: tsc",
            "# MARK: knip"
        ]
    );
}

#[test]
fn pnpm_waits_the_days_of_the_cooldown_in_minutes() {
    let recipes = recipes();
    let node = recipes.recipe("base-node").expect("base-node");
    let cooldown = &node.sections[0];

    let body = text("base-node", &cooldown.template);

    assert_eq!(cooldown.path, "pnpm-workspace.yaml");
    assert_eq!(cooldown.marker, "# MARK: cooldown");
    assert_eq!(parse(&body)["minimumReleaseAge"].as_u64(), Some(10_080));
}

#[test]
fn base_node_adds_biome_eslint_typescript_and_knip_to_the_dev_dependencies_with_pnpm() {
    let recipes = recipes();

    let node = recipes.recipe("base-node").expect("base-node");

    assert_eq!(node.tools.len(), 1);
    assert_eq!(node.tools[0].manager, "pnpm");
    assert_eq!(node.tools[0].group, None);
    assert_eq!(
        node.tools[0].packages,
        [
            "@biomejs/biome",
            "eslint",
            "eslint-plugin-jsdoc",
            "@typescript-eslint/parser",
            "typescript@^6",
            "knip"
        ]
    );
}

#[test]
fn each_hook_runs_a_package_of_the_project_and_never_downloads_it() {
    for (marker, template) in sections() {
        let body = text("base-node", &template);
        assert!(body.starts_with(&format!("{marker}\n")), "{template}");
        let table: toml::Table = body.parse().expect("TOML");
        let hook = &table["repos"][0]["hooks"][0];
        assert_eq!(
            table["repos"][0]["repo"].as_str(),
            Some("local"),
            "{template}"
        );
        assert_eq!(hook["language"].as_str(), Some("system"), "{template}");
        assert!(
            hook["entry"]
                .as_str()
                .is_some_and(|e| e.starts_with("npx --no -- ")),
            "{template}"
        );
    }
}

#[test]
fn biome_writes_its_fixes_and_does_not_fail_on_a_warning() {
    let table: toml::Table = text("base-node", "sections/prek-biome.toml")
        .parse()
        .expect("TOML");
    let entry = table["repos"][0]["hooks"][0]["entry"]
        .as_str()
        .expect("the entry");

    assert!(entry.contains("biome check --write"), "{entry}");
    assert!(!entry.contains("--error-on-warnings"), "{entry}");
}

#[test]
fn biome_reads_the_gitignore_indents_with_spaces_and_fails_on_an_explicit_any() {
    // JSON is YAML, so the reader of the workflows reads it.
    let config = parse(&text("base-node", "files/biome.json"));

    assert_eq!(config["vcs"]["enabled"].as_bool(), Some(true));
    assert_eq!(config["vcs"]["useIgnoreFile"].as_bool(), Some(true));
    assert_eq!(config["formatter"]["indentStyle"].as_str(), Some("space"));
    assert_eq!(
        config["linter"]["rules"]["suspicious"]["noExplicitAny"].as_str(),
        Some("error")
    );
}

#[test]
fn eslint_runs_only_the_three_rules_of_jsdoc() {
    let config = text("base-node", "files/eslint.config.js");
    let rules: Vec<&str> = config
        .lines()
        .filter_map(|line| line.trim().strip_prefix('"'))
        .filter_map(|line| line.split('"').next())
        .collect();

    assert_eq!(
        rules,
        [
            "jsdoc/check-param-names",
            "jsdoc/check-tag-names",
            "jsdoc/require-returns-check"
        ]
    );
    assert!(config.contains("import tsParser from \"@typescript-eslint/parser\";"));
}

#[test]
fn knip_counts_biome_as_used_although_only_prek_runs_it() {
    let config = parse(&text("base-node", "files/knip.json"));

    assert_eq!(
        config["ignoreDependencies"][0].as_str(),
        Some("@biomejs/biome")
    );
}

#[test]
fn full_node_is_base_node_and_ci_node_with_no_step_of_its_own() {
    let recipes = recipes();

    let full = recipes.recipe("full-node").expect("full-node");
    let resolved = recipes.resolve("full-node").expect("resolves");

    assert_eq!(full.extends, ["base-node", "ci-node"]);
    assert!(full.rules.is_empty() && full.files.is_empty() && full.tools.is_empty());
    assert_eq!(
        resolved.chain,
        ["base", "base-node", "ci", "ci-node", "full-node"]
    );
}
