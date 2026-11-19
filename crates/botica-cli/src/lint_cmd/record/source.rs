//! Where a recorded run comes from (ADR 0019 decisions 3 and 6): its origin, the commit under
//! review and the branch, from git and from the variables of CI.

use super::super::git;

/// Who started the run: CI before a hook, so that `prek run` inside a CI job counts as CI.
pub(super) fn origin() -> &'static str {
    let is = |name: &str, value: &str| std::env::var(name).is_ok_and(|found| found == value);
    if is("GITHUB_ACTIONS", "true") || is("GITLAB_CI", "true") {
        "ci"
    } else if is("PRE_COMMIT", "1") {
        "hook"
    } else {
        "cli"
    }
}

/// The commit under review in CI: the head of the pull request on GitHub, the head of the source
/// branch of a pipeline for merged results on GitLab.
pub(super) fn head() -> Option<String> {
    let github = std::env::var("GITHUB_EVENT_NAME")
        .is_ok_and(|event| event.starts_with("pull_request"))
        .then(|| std::env::var("GITHUB_EVENT_PATH").ok())
        .flatten()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|event| {
            event["pull_request"]["head"]["sha"]
                .as_str()
                .map(str::to_owned)
        });
    github
        .or_else(|| set("CI_MERGE_REQUEST_SOURCE_BRANCH_SHA"))
        .filter(|sha| is_object_name(sha))
}

pub(super) fn branch() -> Option<String> {
    set("GITHUB_HEAD_REF")
        .or_else(|| git::answer(&["branch", "--show-current"]))
        .or_else(|| set("CI_MERGE_REQUEST_SOURCE_BRANCH_NAME"))
        .or_else(|| set("CI_COMMIT_REF_NAME"))
}

/// The value of the variable `name`; an empty one counts as unset.
fn set(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

/// The object name of a SHA-1 or a SHA-256 repository.
pub(super) fn is_object_name(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}
