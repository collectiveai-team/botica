//! The `osv-scanner.yml` workflow of `ci-python`.
//!
//! The split of a pull request from the full scan, the `workflow_dispatch` trigger and the tag
//! come from `collectiveai-team/collectiveai-web` (`.github/workflows/osv-scanner.yml`).

mod common;

use common::workflows::{has_trigger, job_using, tag, workflow};
use serde_yaml::Value;

const TEMPLATE: &str = "files/github/workflows/osv-scanner.yml";
const FULL: &str = "osv-scanner-reusable.yml@";
const PULL_REQUEST: &str = "osv-scanner-reusable-pr.yml@";

fn osv() -> Value {
    workflow("ci-python", TEMPLATE)
}

#[test]
fn the_scan_can_be_run_by_hand() {
    assert!(has_trigger(&osv(), "workflow_dispatch"));
}

#[test]
fn main_the_schedule_and_a_manual_run_get_the_full_scan() {
    let osv = osv();

    let full = job_using(&osv, FULL);

    assert_eq!(
        full["if"].as_str(),
        Some("github.event_name != 'pull_request'")
    );
}

#[test]
fn a_pull_request_is_judged_on_what_it_introduces() {
    let osv = osv();

    let pull_request = job_using(&osv, PULL_REQUEST);

    assert_eq!(
        pull_request["if"].as_str(),
        Some("github.event_name == 'pull_request'")
    );
}

#[test]
fn both_scans_pin_the_same_tag_at_or_above_2_6_0() {
    let osv = osv();

    let full = tag(job_using(&osv, FULL)["uses"].as_str().expect("uses"));
    let pull_request = tag(job_using(&osv, PULL_REQUEST)["uses"]
        .as_str()
        .expect("uses"));

    assert_eq!(full, pull_request);
    assert!(full >= vec![2, 6, 0], "pins {full:?}");
}

#[test]
fn both_scans_read_the_lockfile_of_uv() {
    let osv = osv();

    for uses in [FULL, PULL_REQUEST] {
        let args = job_using(&osv, uses)["with"]["scan-args"]
            .as_str()
            .expect("scan-args");

        assert_eq!(args.trim(), "--lockfile=uv.lock", "{uses}");
    }
}

#[test]
fn both_scans_restate_the_permissions_that_the_called_workflow_needs() {
    let osv = osv();

    for uses in [FULL, PULL_REQUEST] {
        let permissions = &job_using(&osv, uses)["permissions"];

        assert_eq!(permissions["actions"].as_str(), Some("read"), "{uses}");
        assert_eq!(permissions["contents"].as_str(), Some("read"), "{uses}");
        assert_eq!(
            permissions["security-events"].as_str(),
            Some("write"),
            "{uses}"
        );
    }
}
