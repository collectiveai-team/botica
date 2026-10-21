//! What the catalog refuses in the fields of a reference: the problems that loading a topic reports.

use crate::references::{parse_lines, Reference, CODE_DIR};

pub(crate) fn check_reference(reference: &Reference) -> Vec<String> {
    let mut problems = Vec::new();
    let targets = [&reference.file, &reference.path, &reference.url]
        .iter()
        .filter(|target| target.is_some())
        .count();
    match targets {
        0 => problems.push("needs a `file`, a `path` or a `url`".to_owned()),
        1 => {}
        _ => problems.push("`file`, `path` and `url` exclude each other".to_owned()),
    }
    if let Some(file) = &reference.file {
        problems.extend(check_topic_file(reference, file));
        return problems;
    }
    if reference
        .path
        .as_deref()
        .is_some_and(|path| path.trim().is_empty())
    {
        problems.push("`path` is empty".to_owned());
    }
    if let Some(url) = &reference.url {
        problems.extend(check_page(reference, url));
        return problems;
    }
    if reference.quote.is_some() {
        problems
            .push("`quote` does not apply to a file reference: the lines are its text".to_owned());
    }
    if let Some(lines) = &reference.lines {
        if parse_lines(lines).is_none() {
            problems.push(format!(
                "`lines` must be `<first>-<last>` or `<line>`, counted from 1 (got `{lines}`)"
            ));
        }
    }
    match (&reference.repo, &reference.rev) {
        (Some(_), None) => {
            problems.push("`repo` needs a `rev`: the commit that fixes the lines".to_owned())
        }
        (None, Some(_)) => problems.push("`rev` needs a `repo`".to_owned()),
        _ => {}
    }
    match (&reference.repo, &reference.license) {
        (Some(_), None) => problems.push("an external reference needs a `license`".to_owned()),
        (None, Some(_)) => problems.push("`license` needs a `repo`".to_owned()),
        _ => {}
    }
    if reference
        .repo
        .as_deref()
        .is_some_and(|repo| !is_repo_address(repo))
    {
        problems.push(
            "`repo` must be `<host>/<path>`, with no scheme and no user (`github.com/<owner>/<repo>`)"
                .to_owned(),
        );
    }
    if reference
        .rev
        .as_deref()
        .is_some_and(|rev| !is_commit_sha(rev))
    {
        problems.push(
            "`rev` must be a full commit SHA: 40 lowercase hex digits (64 in a SHA-256 \
             repository), not a branch, a tag or an abbreviation"
                .to_owned(),
        );
    }
    problems
}

fn check_topic_file(reference: &Reference, file: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let inside = file
        .strip_prefix(&format!("{CODE_DIR}/"))
        .is_some_and(|rest| rest.split('/').all(is_path_segment));
    if !inside {
        problems.push(format!(
            "`file` must be a path under `{CODE_DIR}/` of the topic, like `{CODE_DIR}/core/logger.py` \
             (got `{file}`)"
        ));
    }
    let present = [
        ("repo", reference.repo.is_some()),
        ("rev", reference.rev.is_some()),
        ("license", reference.license.is_some()),
        ("quote", reference.quote.is_some()),
    ];
    for (field, _) in present.iter().filter(|(_, present)| *present) {
        problems.push(format!("`{field}` does not apply to a `file` reference"));
    }
    if let Some(lines) = &reference.lines {
        if parse_lines(lines).is_none() {
            problems.push(format!(
                "`lines` must be `<first>-<last>` or `<line>`, counted from 1 (got `{lines}`)"
            ));
        }
    }
    problems
}

fn check_page(reference: &Reference, url: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !is_http_address(url) {
        problems.push("`url` must be an `https://` or `http://` address".to_owned());
    }
    let present = [
        ("lines", reference.lines.is_some()),
        ("anchor", reference.anchor.is_some()),
        ("repo", reference.repo.is_some()),
        ("rev", reference.rev.is_some()),
        ("license", reference.license.is_some()),
    ];
    for (field, _) in present.iter().filter(|(_, present)| *present) {
        problems.push(format!("`{field}` does not apply to a `url` reference"));
    }
    if reference
        .quote
        .as_deref()
        .is_some_and(|quote| quote.trim().is_empty())
    {
        problems.push("`quote` is empty".to_owned());
    }
    problems
}

fn is_http_address(url: &str) -> bool {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(char::is_whitespace))
}

fn is_commit_sha(rev: &str) -> bool {
    matches!(rev.len(), 40 | 64) && rev.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `<host>/<path>`: a host and at least one segment, with no scheme, no user and no port.
fn is_repo_address(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let host = parts.next().unwrap_or_default();
    let segments: Vec<&str> = parts.collect();
    let host_ok = !host.is_empty()
        && !host.starts_with(['-', '.'])
        && host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'));
    host_ok && !segments.is_empty() && segments.iter().all(|part| is_path_segment(part))
}

fn is_path_segment(part: &str) -> bool {
    !part.is_empty()
        && !matches!(part, "." | "..")
        && !part.starts_with('-')
        && part
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
