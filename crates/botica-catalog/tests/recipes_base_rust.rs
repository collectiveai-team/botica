//! `base-rust`: the rules about Rust code, the cooldown of Cargo, and cargo fmt, clippy and
//! cargo-shear in prek; `ci-rust`: the same checks with test and doc in GitHub Actions, dependabot
//! and osv-scanner; and `full-rust`.

mod common;

use botica_catalog::Recipes;
use common::workflows::{osv_scanner_for, text, workflow};
use serde_yaml::Value;

const RUST: &str = "files/github-rust/workflows/rust.yml";
const DEPENDABOT: &str = "files/github-rust/dependabot.yml";
const OSV: &str = "files/github-rust/workflows/osv-scanner.yml";
const CLIPPY: &str = "cargo clippy --workspace --all-targets --locked -- -D warnings \
                      -W clippy::missing_errors_doc -W clippy::missing_panics_doc";

fn embedded() -> Recipes {
    Recipes::embedded().expect("the built-in recipes load")
}

fn sections() -> Vec<(String, String)> {
    let recipes = embedded();
    let rust = recipes.recipe("base-rust").expect("base-rust");
    rust.sections
        .iter()
        .map(|s| (s.marker.clone(), s.template.clone()))
        .collect()
}

fn section(template: &str) -> toml::Table {
    text("base-rust", template).parse().expect("TOML")
}

#[test]
fn the_recipe_extends_base_and_selects_the_rules_that_read_rust() {
    let recipes = embedded();

    let rust = recipes.recipe("base-rust").expect("base-rust");

    assert_eq!(rust.extends, ["base"]);
    assert_eq!(rust.rules, ["cognitive-complexity", "cargo-cooldown"]);
    assert_eq!(rust.gitignore[0].lines, ["target/"]);
    let markers: Vec<String> = sections().into_iter().map(|s| s.0).collect();
    assert_eq!(markers, ["# MARK: cargo-fmt", "# MARK: cargo-shear"]);
    for (marker, template) in sections() {
        assert!(
            text("base-rust", &template).starts_with(&format!("{marker}\n")),
            "{template}"
        );
    }
}

#[test]
fn cargo_waits_the_days_of_the_cooldown_before_it_takes_a_release() {
    let recipes = embedded();
    let rust = recipes.recipe("base-rust").expect("base-rust");

    let config: toml::Table = text("base-rust", "files/cargo-config.toml")
        .parse()
        .expect("TOML");

    assert_eq!(rust.files[0].path, ".cargo/config.toml");
    assert_eq!(
        config["registry"]["global-min-publish-age"].as_str(),
        Some("7 days")
    );
}

#[test]
fn fmt_rewrites_the_files_and_clippy_fails_on_a_warning_and_an_undocumented_error_or_panic() {
    let table = section("sections/prek-cargo.toml");
    let hooks = table["repos"][0]["hooks"].as_array().expect("the hooks");

    let ids: Vec<&str> = hooks.iter().filter_map(|h| h["id"].as_str()).collect();
    assert_eq!(ids, ["cargo-fmt", "cargo-clippy"]);
    assert_eq!(hooks[0]["entry"].as_str(), Some("cargo fmt --all"));
    assert_eq!(hooks[1]["entry"].as_str(), Some(CLIPPY));
}

#[test]
fn cargo_shear_is_installed_by_prek_at_the_release_of_its_parameter() {
    let table = section("sections/prek-cargo-shear.toml");
    let hook = &table["repos"][0]["hooks"][0];

    assert_eq!(table["repos"][0]["repo"].as_str(), Some("local"));
    assert_eq!(hook["id"].as_str(), Some("cargo-shear"));
    assert_eq!(hook["language"].as_str(), Some("rust"));
    assert_eq!(
        hook["additional_dependencies"][0].as_str(),
        Some("cli:cargo-shear:1.14.0")
    );
}

#[test]
fn ci_rust_runs_the_commands_of_the_hooks_then_test_and_doc() {
    let recipes = embedded();
    let ci = recipes.recipe("ci-rust").expect("ci-rust");
    let rust = workflow("ci-rust", RUST);

    assert_eq!(ci.extends, ["ci"]);
    assert_eq!(ci.rules, ["dependabot-cooldown"]);
    let steps = rust["jobs"]["check"]["steps"]
        .as_sequence()
        .expect("the steps");
    let runs: Vec<&str> = steps.iter().filter_map(|s| s["run"].as_str()).collect();
    assert_eq!(
        runs[1..],
        [
            "cargo fmt --all --check",
            CLIPPY,
            "cargo test --workspace --locked",
            "cargo doc --no-deps --workspace --locked"
        ]
    );
    let doc = steps.last().expect("the step of doc");
    assert_eq!(
        doc["env"]["RUSTDOCFLAGS"].as_str(),
        Some("-D rustdoc::broken_intra_doc_links")
    );
}

#[test]
fn osv_scanner_is_the_one_of_python_with_cargo_lock() {
    let rust = workflow("ci-rust", OSV);
    let python = osv_scanner_for("Cargo.lock");

    for key in ["on", "concurrency", "permissions", "jobs"] {
        assert_eq!(rust[key], python[key], "{key}");
    }
}

#[test]
fn dependabot_keeps_the_actions_and_the_crates_up_to_date_after_a_cooldown() {
    let updates: Vec<Value> = workflow("ci-rust", DEPENDABOT)["updates"]
        .as_sequence()
        .expect("the updates")
        .clone();

    let ecosystems: Vec<&str> = updates
        .iter()
        .map(|u| u["package-ecosystem"].as_str().expect("an ecosystem"))
        .collect();
    assert_eq!(ecosystems, ["github-actions", "cargo"]);
    for update in &updates {
        assert_eq!(update["cooldown"]["default-days"].as_u64(), Some(7));
    }
}

#[test]
fn full_rust_is_base_rust_and_ci_rust_with_no_step_of_its_own() {
    let recipes = embedded();

    let full = recipes.recipe("full-rust").expect("full-rust");
    let resolved = recipes.resolve("full-rust").expect("resolves");

    assert_eq!(full.extends, ["base-rust", "ci-rust"]);
    assert_eq!(
        resolved.chain,
        ["base", "base-rust", "ci", "ci-rust", "full-rust"]
    );
}
