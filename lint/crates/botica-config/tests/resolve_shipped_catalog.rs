//! What the selection makes of the catalog that ships with botica: these lists change with
//! every rule that joins it.

mod common;

use botica_config::Reason;
use common::{active, embedded, reason, resolve};

#[test]
fn the_default_selection_runs_every_rule_enabled_by_default() {
    let catalog = embedded();
    let resolution = resolve(&catalog, "");
    assert_eq!(
        active(&resolution),
        [
            "api-schemas-extra-forbid",
            "arch-database-package",
            "cli-typed-framework",
            "file-size-guard",
            "invalid-suppression",
            "io-error",
            "log-get-logger",
            "log-no-print",
            "no-artifacts-in-package",
            "no-console-log",
            "no-dict-alias",
            "no-dict-call-return",
            "no-dict-literal-return",
            "no-dict-return-annotation",
            "no-notebooks",
            "no-tests-in-package",
            "no-utils",
            "settings-module",
            "unused-suppression"
        ]
    );
    assert_eq!(
        reason(&resolution, "no-utils"),
        Reason::SelectedBy("default".into())
    );
}

#[test]
fn select_and_extend_select_are_added_together() {
    let catalog = embedded();
    assert_eq!(
        active(&resolve(&catalog, "select = [\"BTSTRUC\"]")),
        [
            "file-size-guard",
            "no-artifacts-in-package",
            "no-notebooks",
            "no-tests-in-package",
            "no-utils"
        ]
    );
    let resolution = resolve(
        &catalog,
        "select = [\"no-utils\"]\nextend-select = [\"BT001\"]",
    );
    assert_eq!(active(&resolution), ["io-error", "no-utils"]);
    assert_eq!(reason(&resolution, "file-size-guard"), Reason::NotSelected);
    assert_eq!(
        active(&resolve(&catalog, "select = [\"BT0\"]")),
        ["invalid-suppression", "io-error", "unused-suppression"]
    );
    assert_eq!(
        active(&resolve(&catalog, "select = [\"tag:size\"]")),
        ["file-size-guard"]
    );
    assert_eq!(
        active(&resolve(&catalog, "select = []\nextend-select = [\"all\"]")).len(),
        19
    );
}

#[test]
fn ignore_removes_what_was_selected_and_names_the_selector() {
    let catalog = embedded();
    let resolution = resolve(&catalog, "ignore = [\"BTSTRUC\"]");
    assert_eq!(
        active(&resolution),
        [
            "api-schemas-extra-forbid",
            "arch-database-package",
            "cli-typed-framework",
            "invalid-suppression",
            "io-error",
            "log-get-logger",
            "log-no-print",
            "no-console-log",
            "no-dict-alias",
            "no-dict-call-return",
            "no-dict-literal-return",
            "no-dict-return-annotation",
            "settings-module",
            "unused-suppression"
        ]
    );
    assert_eq!(
        reason(&resolution, "no-utils"),
        Reason::IgnoredBy("BTSTRUC".into())
    );
}

#[test]
fn a_rule_switched_off_in_its_own_section_stays_out() {
    let catalog = embedded();
    let resolution = resolve(
        &catalog,
        "[rules.no-utils]\nenabled = false\n[rules.file-size-guard]\nseverity = \"off\"\n",
    );
    assert_eq!(
        active(&resolution),
        [
            "api-schemas-extra-forbid",
            "arch-database-package",
            "cli-typed-framework",
            "invalid-suppression",
            "io-error",
            "log-get-logger",
            "log-no-print",
            "no-artifacts-in-package",
            "no-console-log",
            "no-dict-alias",
            "no-dict-call-return",
            "no-dict-literal-return",
            "no-dict-return-annotation",
            "no-notebooks",
            "no-tests-in-package",
            "settings-module",
            "unused-suppression"
        ]
    );
    assert_eq!(reason(&resolution, "no-utils"), Reason::Disabled);
    assert_eq!(reason(&resolution, "file-size-guard"), Reason::SeverityOff);
}
