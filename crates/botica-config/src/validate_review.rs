use std::collections::BTreeMap;

use crate::issue::{issue, ConfigIssue};
use crate::matcher::PathMatcher;
use crate::review::{AreaName, ReviewArea, ReviewConfig};
use crate::validate::is_plain_dir;

/// Largest value that `question-max` takes (ADR 0018 decision 8).
const QUESTION_MAX_LIMIT: f64 = 0.3;
/// Smallest value that `impact-min` and `min-confidence` take (ADR 0018 decision 8).
const PROBABILITY_FLOOR: f64 = 0.7;
const MAX_CHANGED_LINES_LIMIT: u32 = 1000;

pub(crate) fn review(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    if let Some(dir) = &config.dir {
        if !is_plain_dir(dir) {
            found.push(issue(
                "review.dir",
                format!("`{dir}` is not a plain directory: write it from the project root, with no leading or trailing `/`, `.` or `..`"),
            ));
        }
    }
    globs(found, "review.generated", &config.generated);
    for (index, area) in config.areas.iter().enumerate() {
        area_entry(found, index, area);
    }
    if !config.areas.is_empty() && !config.areas.iter().any(|a| a.name == AreaName::Platform) {
        found.push(issue(
            "review.areas",
            "there is no `platform` entry: the paths that decide how a pull request is reviewed, built and merged are `platform`, and their owners come from its entries",
        ));
    }
    duplicate_globs(found, &config.areas);
    periphery_over_forced(found, config);
    generated_over_protected(found, config);
    classifier(found, config);
}

fn area_entry(found: &mut Vec<ConfigIssue>, index: usize, area: &ReviewArea) {
    let base = format!("review.areas[{index}]");
    let name = area.name.name();
    if let Some(tier) = area.tier {
        let floor = area.name.floor();
        if tier < floor {
            found.push(issue(
                format!("{base}.tier"),
                format!(
                    "`{}` is under the floor of `{name}`, which is `{}`",
                    tier.name(),
                    floor.name()
                ),
            ));
        }
    }
    if area.name == AreaName::Critical && area.approvals == Some(0) {
        found.push(issue(
            format!("{base}.approvals"),
            "`critical` asks for at least 1 approval",
        ));
    }
    if area.name.needs_owners() && area.owners.is_empty() {
        found.push(issue(
            format!("{base}.owners"),
            format!("`{name}` needs the people or teams that answer for its paths"),
        ));
    }
    for (owner_index, owner) in area.owners.iter().enumerate() {
        if !is_owner(owner) {
            found.push(issue(
                format!("{base}.owners[{owner_index}]"),
                format!("`{owner}` is not an owner: write `@user`, `@org/team` or an email address, with no blanks"),
            ));
        }
    }
    if area.paths.is_empty() {
        found.push(issue(
            format!("{base}.paths"),
            "must list at least one glob",
        ));
    }
    globs(found, &format!("{base}.paths"), &area.paths);
}

fn is_owner(owner: &str) -> bool {
    owner.contains('@') && !owner.chars().any(char::is_whitespace)
}

/// A glob of the table, which a CODEOWNERS file and `applies.paths` read as well: a line that is
/// blank, a comment or a negation would mean something else there.
fn globs(found: &mut Vec<ConfigIssue>, key: &str, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        let location = format!("{key}[{index}]");
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
            found.push(issue(
                location,
                format!("`{value}` is not a glob of paths: it must not be blank or start with `#` or `!`"),
            ));
        } else if value.chars().any(char::is_control) {
            found.push(issue(location, "a glob has no control characters"));
        } else if let Err(error) = PathMatcher::check(value) {
            found.push(issue(location, format!("invalid glob `{value}`: {error}")));
        }
    }
}

/// A glob that two entries share would give one path two owners at once, and the export of CODEOWNERS
/// keeps one line for it.
fn duplicate_globs(found: &mut Vec<ConfigIssue>, areas: &[ReviewArea]) {
    let mut seen: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (index, area) in areas.iter().enumerate() {
        for (path_index, path) in area.paths.iter().enumerate() {
            if let Some((first, first_path)) = seen.insert(path.trim(), (index, path_index)) {
                found.push(issue(
                    format!("review.areas[{index}].paths[{path_index}]"),
                    format!("`{path}` is already in review.areas[{first}].paths[{first_path}]"),
                ));
            }
        }
    }
}

/// A concrete path that the glob selects, made by writing a name where it has a wildcard.
fn sample(glob: &str) -> String {
    let glob = glob.trim().trim_start_matches('/').trim_end_matches('/');
    let mut out = String::new();
    let mut chars = glob.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                while chars.next_if_eq(&'*').is_some() {}
                out.push('x');
            }
            '?' => out.push('x'),
            '[' => {
                let mut class = chars.by_ref().take_while(|&c| c != ']');
                let first = class.find(|&c| c != '!' && c != '^').unwrap_or('x');
                out.push(first);
            }
            '{' => {
                let alternative: String = chars
                    .by_ref()
                    .take_while(|&c| c != '}')
                    .take_while(|&c| c != ',')
                    .collect();
                out.push_str(&alternative);
            }
            '\\' => out.extend(chars.next()),
            other => out.push(other),
        }
    }
    out
}

/// The samples of a glob, and the shallowest one when it starts with `**/`.
fn samples(glob: &str) -> Vec<String> {
    let mut all = vec![sample(glob)];
    if let Some(rest) = glob.trim().strip_prefix("**/") {
        all.push(sample(rest));
    }
    all
}

fn matcher(glob: &str) -> Option<PathMatcher> {
    PathMatcher::new(&[glob.to_owned()]).ok()
}

/// The paths that are `platform` whatever the entries say, as one glob and one sample each.
fn forced(config: &ReviewConfig) -> Vec<(String, Vec<String>)> {
    config
        .forced_platform_globs()
        .into_iter()
        .map(|glob| {
            let paths = samples(&glob);
            (glob, paths)
        })
        .collect()
}

/// `botica.toml` fails to load when an entry of `periphery` covers a forced path (ADR 0018
/// decision 1). Whether two globs overlap is decided on a sample path of each, so a glob that
/// overlaps only at paths that no sample reaches goes unnoticed.
fn periphery_over_forced(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    let forced = forced(config);
    for (index, area) in config.areas.iter().enumerate() {
        if area.name != AreaName::Periphery {
            continue;
        }
        for (path_index, glob) in area.paths.iter().enumerate() {
            let Some(matcher) = matcher(glob) else {
                continue;
            };
            let covered = forced.iter().find_map(|(rule, paths)| {
                paths
                    .iter()
                    .find(|path| matcher.is_match(path))
                    .map(|path| (rule, path))
            });
            if let Some((rule, path)) = covered {
                found.push(issue(
                    format!("review.areas[{index}].paths[{path_index}]"),
                    format!("`{glob}` covers `{path}`, which is `platform` whatever the entries say (`{rule}`): leave it to a `platform` entry"),
                ));
            }
        }
    }
}

/// `botica.toml` fails to load when a glob of `generated` matches a path of `critical`, of
/// `platform` or of the forced list, since the size of a change would not count the files that
/// a person has to read.
fn generated_over_protected(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    let mut protected = forced(config);
    for area in &config.areas {
        if matches!(area.name, AreaName::Critical | AreaName::Platform) {
            for glob in &area.paths {
                protected.push((glob.clone(), samples(glob)));
            }
        }
    }
    for (index, glob) in config.generated.iter().enumerate() {
        let Some(generated) = matcher(glob) else {
            continue;
        };
        let own = samples(glob);
        let clash = protected.iter().find(|(rule, paths)| {
            paths.iter().any(|path| generated.is_match(path))
                || matcher(rule).is_some_and(|m| own.iter().any(|path| m.is_match(path)))
        });
        if let Some((rule, _)) = clash {
            found.push(issue(
                format!("review.generated[{index}]"),
                format!("`{glob}` matches `{rule}`, a path of `critical` or `platform` that a person has to review: a change to it counts in full"),
            ));
        }
    }
}

fn classifier(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    let settings = &config.classifier;
    for (index, check) in settings.required_checks.iter().enumerate() {
        if check.trim().is_empty() {
            found.push(issue(
                format!("review.classifier.required-checks[{index}]"),
                "a check has a name",
            ));
        }
    }
    if !(0.0..=QUESTION_MAX_LIMIT).contains(&settings.question_max) {
        found.push(issue(
            "review.classifier.question-max",
            format!("must be from 0 to {QUESTION_MAX_LIMIT}"),
        ));
    }
    for (key, value) in [
        ("impact-min", settings.impact_min),
        ("min-confidence", settings.min_confidence),
    ] {
        if !(PROBABILITY_FLOOR..=1.0).contains(&value) {
            found.push(issue(
                format!("review.classifier.{key}"),
                format!("must be from {PROBABILITY_FLOOR} to 1"),
            ));
        }
    }
    if settings.max_changed_lines > MAX_CHANGED_LINES_LIMIT {
        found.push(issue(
            "review.classifier.max-changed-lines",
            format!("must be at most {MAX_CHANGED_LINES_LIMIT}"),
        ));
    }
}
