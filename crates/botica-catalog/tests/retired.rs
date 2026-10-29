//! The retired names of the built-in catalog resolve as ADR 0016 says (decisions 2 and 5).

use botica_catalog::{Catalog, RetiredKind};

fn embedded() -> Catalog {
    Catalog::embedded().unwrap_or_else(|error| panic!("the built-in catalog is invalid:\n{error}"))
}

fn named(catalog: &Catalog, name: &str) -> Option<String> {
    catalog.resolve(name).map(|rule| rule.id.clone())
}

#[test]
fn a_retired_code_that_no_rule_holds_names_the_last_rule_that_held_it() {
    let catalog = embedded();

    for (code, rule) in [
        ("BTARCH005", "db-no-create-all"),
        ("BTARCH006", "settings-env-schema"),
        ("BTARCH007", "import-layers"),
        ("BTDM005", "no-dict-return-annotation"),
    ] {
        assert_eq!(named(&catalog, code).as_deref(), Some(rule), "{code}");
        let retired = catalog.retired(code).expect("a retired code");
        assert_eq!(retired.kind, RetiredKind::Code, "{code}");
        assert_eq!(retired.since, "0.13.0", "{code}");
    }
}

#[test]
fn a_code_that_a_rule_holds_names_that_rule_and_is_not_retired() {
    let catalog = embedded();

    for (code, rule) in [
        ("BTPROC001", "pr-small-by-intent"),
        ("BTARCH001", "import-layers"),
        ("BTARCH004", "no-eager-heavy-imports"),
        ("BTDM001", "no-dict-alias"),
        ("BTLOG003", "no-console-log"),
    ] {
        assert_eq!(named(&catalog, code).as_deref(), Some(rule), "{code}");
        assert!(catalog.retired(code).is_none(), "{code}");
    }
}

#[test]
fn the_old_id_of_a_renamed_rule_is_retired_and_names_it() {
    let catalog = embedded();

    assert_eq!(
        named(&catalog, "arch-database-package").as_deref(),
        Some("database-package")
    );
    assert_eq!(
        catalog.retired_warning("arch-database-package").as_deref(),
        Some("`arch-database-package` is the old id of `database-package` (database-package since 0.13.0)")
    );
}

#[test]
fn the_warning_of_a_retired_code_names_the_rule_its_code_and_the_release() {
    assert_eq!(
        embedded().retired_warning("BTARCH005").as_deref(),
        Some("`BTARCH005` is the old code of `db-no-create-all` (BTDB002 since 0.13.0)")
    );
}

#[test]
fn no_retired_name_is_the_id_the_code_or_an_alias_of_another_rule() {
    let catalog = embedded();

    for rule in catalog.rules() {
        let names = [&rule.id, &rule.code].into_iter().chain(&rule.aliases);
        for name in names {
            if let Some(retired) = catalog.retired(name) {
                assert_eq!(
                    retired.rule, rule.id,
                    "`{name}` is retired from another rule"
                );
            }
        }
    }
}

#[test]
fn a_removed_rule_says_the_release_that_removed_it() {
    let catalog = embedded();
    let removed = catalog.resolve("no-console-log").expect("the rule stays");

    assert_eq!(catalog.removed_in(removed).as_deref(), Some("0.10.0"));
    let live = catalog.resolve("log-no-print").expect("a rule");
    assert_eq!(catalog.removed_in(live), None);
}
