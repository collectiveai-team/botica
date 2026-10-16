//! Whether the tests that run a real `node` can run here.

use std::process::Command;

/// Whether `node` runs. A test that needs it returns when it does not, unless
/// `BOTICA_REQUIRE_NODE` is set (CI): then a missing `node` fails the test, so a runner without it
/// cannot pass for a run that checked the scripts.
pub fn installed() -> bool {
    let found = Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !found {
        assert!(
            std::env::var_os("BOTICA_REQUIRE_NODE").is_none(),
            "node is not installed and BOTICA_REQUIRE_NODE is set"
        );
        eprintln!("node is not installed: skipped");
    }
    found
}
