use std::path::Path;

use botica_conformance::Runner;

#[test]
fn every_rule_of_the_catalog_conforms_to_its_fixtures_with_and_without_each_optimisation() {
    let rules = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules");
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("conformance");
    let outcomes = Runner::new(env!("CARGO_BIN_EXE_botica"), &scratch)
        .check_catalog(&rules)
        .expect("the catalog is checked");

    let names: Vec<&str> = outcomes.iter().map(|outcome| outcome.rule()).collect();
    assert_eq!(
        names,
        [
            "api-request-schema-name",
            "api-response-schema-name",
            "api-schema-in-route-module",
            "api-schemas-extra-forbid",
            "arch-database-package",
            "cli-typed-framework",
            "db-no-create-all",
            "external-config",
            "file-size-guard",
            "import-layers",
            "invalid-suppression",
            "io-error",
            "log-get-logger",
            "log-no-print",
            "no-artifacts-in-package",
            "no-dict-alias",
            "no-dict-call-return",
            "no-dict-literal-return",
            "no-dict-return-annotation",
            "no-notebooks",
            "no-tests-in-package",
            "no-utils",
            "settings-base-class",
            "settings-direct-instantiation",
            "settings-env-schema",
            "settings-module",
            "unused-suppression"
        ]
    );
    let report: Vec<String> = outcomes
        .iter()
        .filter(|outcome| !outcome.passed())
        .map(|outcome| {
            let reasons: Vec<String> = outcome.failures().iter().map(ToString::to_string).collect();
            format!("{}:\n{}", outcome.rule(), reasons.join("\n"))
        })
        .collect();
    assert!(report.is_empty(), "{}", report.join("\n\n"));
}
