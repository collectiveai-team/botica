//! The workflows that the built-in recipes of CI add, read as YAML.

use botica_catalog::Recipes;
use serde_yaml::Value;

pub fn workflow(recipe: &str, template: &str) -> Value {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let text = recipes
        .template(recipe, template)
        .unwrap_or_else(|| panic!("{recipe} has no template {template}"));
    serde_yaml::from_str(text).unwrap_or_else(|error| panic!("{template} is not YAML: {error}"))
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
