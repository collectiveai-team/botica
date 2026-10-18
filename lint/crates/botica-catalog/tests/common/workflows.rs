//! The workflows that the built-in recipes of CI add, read as YAML.

use botica_catalog::Recipes;
use serde_yaml::Value;

pub fn text(recipe: &str, template: &str) -> String {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    recipes
        .template(recipe, template)
        .unwrap_or_else(|| panic!("{recipe} has no template {template}"))
        .to_owned()
}

pub fn parse(text: &str) -> Value {
    serde_yaml::from_str(text).unwrap_or_else(|error| panic!("not YAML: {error}"))
}

pub fn workflow(recipe: &str, template: &str) -> Value {
    parse(&text(recipe, template))
}

/// The scan of `ci-python` with its lockfile replaced: what the scan of another ecosystem is.
pub fn osv_scanner_for(lockfile: &str) -> Value {
    parse(
        &text("ci-python", "files/github/workflows/osv-scanner.yml")
            .replace("--lockfile=uv.lock", &format!("--lockfile={lockfile}")),
    )
}

/// The job of `workflow` whose `uses:` contains `uses`.
pub fn job_using<'a>(workflow: &'a Value, uses: &str) -> &'a Value {
    let jobs = workflow["jobs"].as_mapping().expect("the jobs");
    jobs.values()
        .find(|job| job["uses"].as_str().is_some_and(|u| u.contains(uses)))
        .unwrap_or_else(|| panic!("no job uses {uses}"))
}

pub fn has_trigger(workflow: &Value, trigger: &str) -> bool {
    workflow["on"]
        .as_mapping()
        .is_some_and(|on| on.contains_key(trigger))
}

/// The tag of an `uses: <path>@<tag>` reference, as numbers.
pub fn tag(uses: &str) -> Vec<u32> {
    uses.rsplit('@')
        .next()
        .expect("a reference")
        .trim_start_matches('v')
        .split('.')
        .map(|part| part.parse().expect("a number"))
        .collect()
}
