//! `ci-go`: the checks of a Go module, dependabot and osv-scanner.
//!
//! `go.yml` comes from `collectiveai-team/orquesta-lite` 3fe6139 (`.github/workflows/go.yml`):
//! the order of the checks, `-race`, and a `pull_request` with no branch filter, because that
//! repository lands work as stacks of pull requests. The pins of the actions, `persist-credentials`
//! and the permissions are the ones of the other templates of botica, which that workflow lacks.

mod common;

use botica_catalog::Recipes;
use common::workflows::{osv_scanner_for, tag, workflow};
use serde_yaml::Value;

const GO: &str = "files/github-go/workflows/go.yml";
const DEPENDABOT: &str = "files/github-go/dependabot.yml";
const OSV: &str = "files/github-go/workflows/osv-scanner.yml";

fn steps(go: &Value) -> Vec<Value> {
    go["jobs"]["check"]["steps"]
        .as_sequence()
        .expect("the steps")
        .clone()
}

fn named<'a>(steps: &'a [Value], name: &str) -> &'a Value {
    steps
        .iter()
        .find(|step| step["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("no step named {name}"))
}

#[test]
fn the_recipe_adds_the_checks_dependabot_and_osv_scanner_next_to_ci() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");

    let go = recipes.recipe("ci-go").expect("ci-go");

    let paths: Vec<&str> = go.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            ".github/workflows/go.yml",
            ".github/dependabot.yml",
            ".github/workflows/osv-scanner.yml"
        ]
    );
    assert_eq!(go.extends, ["ci"]);
}

#[test]
fn the_checks_run_on_main_and_on_every_pull_request_whatever_its_base() {
    let go = workflow("ci-go", GO);

    assert_eq!(go["on"]["push"]["branches"][0].as_str(), Some("main"));
    let pull_request = &go["on"]["pull_request"];
    assert!(pull_request.is_null(), "{pull_request:?}");
}

#[test]
fn the_checks_run_in_the_order_format_build_vet_test() {
    let go = workflow("ci-go", GO);
    let steps = steps(&go);

    let names: Vec<&str> = steps
        .iter()
        .filter_map(|step| step["name"].as_str())
        .collect();

    assert_eq!(names, ["gofmt", "build", "vet", "test"]);
    assert_eq!(
        named(&steps, "build")["run"].as_str(),
        Some("go build ./...")
    );
    assert_eq!(named(&steps, "vet")["run"].as_str(), Some("go vet ./..."));
    assert_eq!(
        named(&steps, "test")["run"].as_str(),
        Some("go test ./... -count=1 -race")
    );
}

#[test]
fn gofmt_prints_the_diff_and_not_only_the_names_of_the_files() {
    let go = workflow("ci-go", GO);
    let steps = steps(&go);

    let script = named(&steps, "gofmt")["run"].as_str().expect("run");

    assert!(script.contains("gofmt -l ."), "{script}");
    assert!(script.contains("gofmt -d"), "{script}");
    assert!(script.contains("exit 1"), "{script}");
}

#[test]
fn the_go_version_is_the_one_of_the_module() {
    let go = workflow("ci-go", GO);
    let steps = steps(&go);

    let setup = steps
        .iter()
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/setup-go@"))
        })
        .expect("setup-go");

    assert_eq!(setup["with"]["go-version-file"].as_str(), Some("go.mod"));
    let pinned = tag(setup["uses"].as_str().expect("uses"));
    assert!(pinned >= vec![7], "{pinned:?}");
}

#[test]
fn the_job_follows_the_conventions_of_the_other_workflows() {
    let go = workflow("ci-go", GO);
    let steps = steps(&go);

    assert_eq!(go["permissions"]["contents"].as_str(), Some("read"));
    assert!(go["concurrency"]["cancel-in-progress"]
        .as_bool()
        .expect("a flag"));
    let checkout = steps
        .iter()
        .find(|s| {
            s["uses"]
                .as_str()
                .is_some_and(|u| u.starts_with("actions/checkout@"))
        })
        .expect("checkout");
    assert!(tag(checkout["uses"].as_str().expect("uses")) >= vec![7]);
    assert_eq!(
        checkout["with"]["persist-credentials"].as_bool(),
        Some(false)
    );
}

#[test]
fn dependabot_keeps_the_actions_and_the_go_modules_up_to_date() {
    let updates = workflow("ci-go", DEPENDABOT)["updates"]
        .as_sequence()
        .expect("the updates")
        .clone();

    let ecosystems: Vec<&str> = updates
        .iter()
        .map(|u| u["package-ecosystem"].as_str().expect("an ecosystem"))
        .collect();
    assert_eq!(ecosystems, ["github-actions", "gomod"]);
    for update in &updates {
        assert_eq!(update["directory"].as_str(), Some("/"));
        assert_eq!(update["schedule"]["interval"].as_str(), Some("weekly"));
        assert_eq!(update["cooldown"]["default-days"].as_u64(), Some(7));
    }
}

#[test]
fn osv_scanner_is_the_one_of_python_with_go_mod() {
    let go = workflow("ci-go", OSV);
    let python = osv_scanner_for("go.mod");

    for key in ["on", "concurrency", "permissions", "jobs"] {
        assert_eq!(go[key], python[key], "{key}");
    }
    let args = go["jobs"]["osv-scanner"]["with"]["scan-args"].as_str();
    assert_eq!(args.map(str::trim), Some("--lockfile=go.mod"));
}
