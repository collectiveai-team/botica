//! `base-go`: the rules about Go code, `.golangci.yml`, golangci-lint, `go mod tidy` and govulncheck
//! in prek, and `full-go`.

mod common;

use botica_catalog::Recipes;
use common::workflows::{parse, text};

fn sections() -> Vec<(String, String)> {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let go = recipes.recipe("base-go").expect("base-go");
    go.sections
        .iter()
        .map(|s| (s.marker.clone(), s.template.clone()))
        .collect()
}

#[test]
fn the_recipe_extends_base_and_selects_the_rules_that_read_go() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");

    let go = recipes.recipe("base-go").expect("base-go");

    assert_eq!(go.extends, ["base"]);
    assert_eq!(go.rules, ["cognitive-complexity"]);
    let paths: Vec<&str> = go.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, [".golangci.yml"]);
    assert_eq!(
        sections(),
        [
            (
                "# MARK: golangci-lint".to_owned(),
                "sections/prek-golangci-lint.toml".to_owned()
            ),
            (
                "# MARK: go-mod-tidy".to_owned(),
                "sections/prek-go-mod-tidy.toml".to_owned()
            ),
            (
                "# MARK: govulncheck".to_owned(),
                "sections/prek-govulncheck.toml".to_owned()
            ),
        ]
    );
}

#[test]
fn each_section_starts_with_its_marker_and_is_a_table_of_repos() {
    for (marker, template) in sections() {
        let section = text("base-go", &template);
        assert!(section.starts_with(&format!("{marker}\n")), "{template}");
        let table: toml::Table = section.parse().expect("TOML");
        assert!(table["repos"].is_array(), "{template}");
    }
}

#[test]
fn golangci_lint_runs_from_a_pinned_tag_with_the_go_of_the_path() {
    let section: toml::Table = text("base-go", "sections/prek-golangci-lint.toml")
        .parse()
        .expect("TOML");
    let repo = &section["repos"][0];

    assert_eq!(
        repo["repo"].as_str(),
        Some("https://github.com/golangci/golangci-lint")
    );
    assert!(repo["rev"]
        .as_str()
        .is_some_and(|rev| rev.starts_with("v2.")));
    let hooks = repo["hooks"].as_array().expect("the hooks");
    let ids: Vec<&str> = hooks.iter().filter_map(|h| h["id"].as_str()).collect();
    assert_eq!(
        ids,
        [
            "golangci-lint-config-verify",
            "golangci-lint-fmt",
            "golangci-lint",
            "golangci-lint-full"
        ]
    );
    let unused: Vec<&str> = hooks[3]["args"]
        .as_array()
        .expect("the args of the hook of unused")
        .iter()
        .filter_map(|a| a.as_str())
        .collect();
    assert_eq!(unused, ["--enable-only", "unused"]);
    for hook in hooks {
        assert_eq!(
            hook["language_version"]["preference"].as_str(),
            Some("system"),
            "{hook:?}"
        );
    }
}

#[test]
fn the_configuration_is_version_2_with_gofumpt_and_without_a_complexity_linter() {
    let config = parse(&text("base-go", "files/golangci.yml"));

    assert_eq!(config["version"].as_str(), Some("2"));
    assert_eq!(config["linters"]["default"].as_str(), Some("standard"));
    let enabled: Vec<&str> = config["linters"]["enable"]
        .as_sequence()
        .expect("the linters")
        .iter()
        .filter_map(|l| l.as_str())
        .collect();
    assert_eq!(
        enabled,
        [
            "errorlint",
            "wrapcheck",
            "bodyclose",
            "gosec",
            "revive",
            "godoclint"
        ]
    );
    assert_eq!(
        config["linters"]["settings"]["revive"]["rules"][0]["name"].as_str(),
        Some("package-naming")
    );
    assert_eq!(config["formatters"]["enable"][0].as_str(), Some("gofumpt"));
}

#[test]
fn the_go_of_prek_and_the_stage_and_release_of_govulncheck_are_parameters() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let params = &recipes.recipe("base-go").expect("base-go").params;

    let defaults: Vec<(&str, &str)> = params
        .iter()
        .map(|(name, spec)| (name.as_str(), spec.default.as_str()))
        .collect();
    assert_eq!(
        defaults,
        [
            ("go-preference", "system"),
            ("govulncheck-stage", "manual"),
            ("govulncheck-version", "v1.9.0")
        ]
    );
    assert_eq!(
        params["go-preference"].choices,
        ["system", "only-system", "managed", "only-managed"]
    );
    let raw = recipes
        .template("base-go", "sections/prek-golangci-lint.toml")
        .expect("the template");
    assert_eq!(raw.matches("{{ param.go-preference }}").count(), 4);
}

#[test]
fn govulncheck_is_in_the_manual_stage_and_runs_a_pinned_release() {
    let section: toml::Table = text("base-go", "sections/prek-govulncheck.toml")
        .parse()
        .expect("TOML");
    let hook = &section["repos"][0]["hooks"][0];

    assert_eq!(hook["id"].as_str(), Some("govulncheck"));
    assert_eq!(hook["stages"][0].as_str(), Some("manual"));
    assert!(hook["entry"]
        .as_str()
        .is_some_and(|e| e.starts_with("go run golang.org/x/vuln/cmd/govulncheck@v1.")));
}

#[test]
fn full_go_is_base_go_and_ci_go_with_no_step_of_its_own() {
    let recipes = Recipes::embedded().expect("the built-in recipes load");

    let full = recipes.recipe("full-go").expect("full-go");
    let resolved = recipes.resolve("full-go").expect("resolves");

    assert_eq!(full.extends, ["base-go", "ci-go"]);
    assert!(full.rules.is_empty() && full.files.is_empty() && full.sections.is_empty());
    assert_eq!(
        resolved.chain,
        ["base", "base-go", "ci", "ci-go", "full-go"]
    );
}
