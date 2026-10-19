//! What `zizmor` 1.31.1 reports on the workflows of the recipes of CI, when all of them are
//! installed in one repository (`zizmor --strict-collection`, the command of `zizmor.yml`).
//!
//! It reported a `low` finding (`self-repository`) in `release.yml`.

mod common;

use common::workflows::workflow;

#[test]
fn the_release_calls_the_tests_of_its_own_repository_with_the_self_repository_syntax() {
    let release = workflow("ci-publish", "files/github/workflows/release.yml");

    assert_eq!(
        release["jobs"]["test"]["uses"].as_str(),
        Some("$/.github/workflows/tests.yml")
    );
}
