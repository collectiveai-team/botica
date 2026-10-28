//! The version of each tool that a built-in template runs with `uvx` or `npx` is a parameter, and
//! its default is an exact version, which `scripts/bump_tool_params.py` moves.

use std::collections::BTreeMap;

use botica_catalog::{render, Recipes};

/// The text that an agent reads, not a command that a hook or a workflow runs: it asks for the
/// botica of the `PATH` and has to say the same as the `AGENTS.md` of this repository.
const PROSE: &str = "sections/botica-rules.md";

/// Each `(recipe, template, text)` of the steps of the built-in recipes, once per template.
fn templates(recipes: &Recipes) -> Vec<(String, String, String)> {
    let mut seen = BTreeMap::new();
    for recipe in recipes.recipes() {
        let paths = recipe
            .files
            .iter()
            .map(|step| &step.template)
            .chain(recipe.sections.iter().map(|step| &step.template));
        for path in paths {
            let text = recipes.template(&recipe.id, path).expect("a template");
            seen.entry(path.clone())
                .or_insert_with(|| (recipe.id.clone(), text.to_owned()));
        }
    }
    seen.into_iter()
        .map(|(path, (recipe, text))| (recipe, path, text))
        .collect()
}

/// The word after `uvx ` or `npx --yes `: the package that the command runs.
fn packages(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    for runner in ["uvx ", "npx --yes "] {
        for (at, _) in text.match_indices(runner) {
            let rest = &text[at + runner.len()..];
            let end = rest.find([' ', '"', '`', '\n']).unwrap_or(rest.len());
            found.push(&rest[..end]);
        }
    }
    found
}

#[test]
fn every_tool_that_a_built_in_template_runs_with_uvx_or_npx_reads_its_version_from_a_parameter() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");

    for (_, path, text) in templates(&recipes)
        .into_iter()
        .filter(|(_, path, _)| path != PROSE)
    {
        for package in packages(&text) {
            assert!(
                package.contains("{{"),
                "`{path}` runs `{package}` without a parameter for its version"
            );
        }
    }
}

/// `name@1.2.3`: a package at an exact version, as `scripts/bump_tool_params.py` writes it.
fn pinned(package: &str) -> bool {
    package.rsplit_once('@').is_some_and(|(name, version)| {
        !name.is_empty()
            && version.split('.').count() == 3
            && version
                .split('.')
                .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    })
}

#[test]
fn the_defaults_pin_each_tool_to_an_exact_version() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");

    for (recipe, path, text) in templates(&recipes)
        .into_iter()
        .filter(|(_, path, _)| path != PROSE)
    {
        let resolved = recipes.resolve(&recipe).expect("resolves");
        let written = render(&text, &resolved.params).expect("renders");
        for package in packages(&written) {
            assert!(pinned(package), "`{path}` runs `{package}`");
        }
    }
}

#[test]
fn the_rev_of_the_mirror_of_botica_is_the_release_of_the_binary() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let text = recipes
        .template("base", "files/prek-base.toml")
        .expect("the template");
    let resolved = recipes.resolve("base").expect("resolves");

    let written = render(text, &resolved.params).expect("renders");

    let mirror = "repo = \"https://github.com/collectiveai-team/botica-pre-commit\"\n";
    let rev = written
        .split(mirror)
        .nth(1)
        .and_then(|rest| rest.lines().next());
    assert_eq!(
        rev,
        Some(format!("rev = \"v{}\"", env!("CARGO_PKG_VERSION")).as_str())
    );
}
