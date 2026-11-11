use std::collections::BTreeMap;

use crate::glob_overlap::Glob;
use crate::issue::{issue, ConfigIssue};
use crate::matcher::{is_plain_dir, PathMatcher};
use crate::review::{AreaName, ReviewArea, ReviewConfig};

/// Largest value that `question-max` takes (ADR 0018 decision 8).
const QUESTION_MAX_LIMIT: f64 = 0.3;
/// Smallest value that `impact-min` and `min-confidence` take (ADR 0018 decision 8).
const PROBABILITY_FLOOR: f64 = 0.7;
const MAX_CHANGED_LINES_LIMIT: u32 = 1000;

pub(crate) fn review(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    directory(found, config.dir.as_deref());
    globs(found, "review.generated", &config.generated);
    for (index, area) in config.areas.iter().enumerate() {
        area_entry(found, index, area);
    }
    platform_entry(found, config);
    duplicate_globs(found, &config.areas);
    periphery_over_forced(found, config);
    generated_over_protected(found, config);
    classifier(found, config);
}

fn directory(found: &mut Vec<ConfigIssue>, dir: Option<&str>) {
    let Some(dir) = dir else {
        return;
    };
    let wildcard = dir.contains(['*', '?', '[', '{', '\\']) || dir.starts_with(['!', '#']);
    if !is_plain_dir(dir) || wildcard {
        found.push(issue(
            "review.dir",
            format!("`{dir}` is not a plain directory: write it from the project root, with no leading or trailing `/`, `.` or `..` and no wildcard"),
        ));
    }
}

fn platform_entry(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    let has_platform = config.areas.iter().any(|a| a.name == AreaName::Platform);
    if !config.areas.is_empty() && !has_platform {
        found.push(issue(
            "review.areas",
            "there is no `platform` entry: the paths that decide how a pull request is reviewed, built and merged are `platform`, and their owners come from its entries",
        ));
    }
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
    if area.name != AreaName::Periphery && area.approvals == Some(0) {
        found.push(issue(
            format!("{base}.approvals"),
            format!("`{name}` asks for at least 1 approval; only `periphery` takes 0"),
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
                format!(
                    "`{owner}` is not an owner: write `@user`, `@org/team` or an email address"
                ),
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

fn handle_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

/// What CODEOWNERS reads as an owner: `@user`, `@org/team` or an email address.
fn is_owner(owner: &str) -> bool {
    if let Some(handle) = owner.strip_prefix('@') {
        let mut parts = handle.split('/');
        let valid =
            |part: Option<&str>| part.is_some_and(|p| !p.is_empty() && p.chars().all(handle_char));
        let user = parts.next();
        let team = parts.next();
        return valid(user) && (team.is_none() || valid(team)) && parts.next().is_none();
    }
    let Some((local, domain)) = owner.split_once('@') else {
        return false;
    };
    let plain = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.+".contains(c))
    };
    plain(local) && plain(domain) && domain.contains('.')
}

/// A glob of the table, which a CODEOWNERS file and `applies.paths` read as well: a line that is
/// blank, a comment or a negation would mean something else there, and spaces around a glob would
/// be part of the glob for the matcher.
fn globs(found: &mut Vec<ConfigIssue>, key: &str, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        let location = format!("{key}[{index}]");
        if value.trim().is_empty() || value.starts_with(['#', '!']) {
            found.push(issue(
                location,
                format!("`{value}` is not a glob of paths: it must not be blank or start with `#` or `!`"),
            ));
        } else if value != value.trim() {
            found.push(issue(
                location,
                format!("`{value}` has spaces around it: a space is part of the glob"),
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
            if let Some((first, first_path)) = seen.insert(path, (index, path_index)) {
                found.push(issue(
                    format!("review.areas[{index}].paths[{path_index}]"),
                    format!("`{path}` is already in review.areas[{first}].paths[{first_path}]"),
                ));
            }
        }
    }
}

/// The paths that are `platform` whatever the entries say, as the shapes they come in: a folder at
/// the root (`x/**`), a name or a folder at any depth (`**/x`, `**/x/**`) and a name at the root.
enum Forced {
    Folder,
    AtAnyDepth,
    NameAtRoot,
}

impl Forced {
    fn of(glob: &str) -> Self {
        if glob.starts_with("**/") {
            Self::AtAnyDepth
        } else if glob.ends_with("/**") {
            Self::Folder
        } else {
            Self::NameAtRoot
        }
    }

    /// The glob as the overlap reads it: a name of the root is a file, anchored with a leading `/`.
    fn parsed(glob: &str) -> Glob {
        match Self::of(glob) {
            Self::NameAtRoot if !glob.starts_with('/') => Glob::file(&format!("/{glob}")),
            Self::NameAtRoot | Self::AtAnyDepth => Glob::file(glob),
            Self::Folder => Glob::parse(glob),
        }
    }
}

/// `botica.toml` fails to load when an entry of `periphery` covers a forced path (ADR 0018
/// decision 1). An entry covers a folder when it selects a path inside it, and a name when it
/// picks files by name and can pick that one. An entry that ends in `**`, as `docs/**`, picks a
/// folder and not names, so a forced file inside it stays `platform` by the list and is not an
/// error. A name with no `/` is read at every depth, as in a `.gitignore`: `README.md` selects
/// `.github/review/README.md` too, and `/README.md` is the file at the root.
fn periphery_over_forced(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    let forced: Vec<(String, Forced, Glob)> = config
        .forced_platform_globs()
        .into_iter()
        .map(|glob| {
            let parsed = Forced::parsed(&glob);
            let shape = Forced::of(&glob);
            (glob, shape, parsed)
        })
        .collect();
    for (index, area) in config.areas.iter().enumerate() {
        if area.name != AreaName::Periphery {
            continue;
        }
        for (path_index, text) in area.paths.iter().enumerate() {
            let glob = Glob::parse(text);
            let covered = forced.iter().find(|(rule, shape, forced)| match shape {
                Forced::AtAnyDepth => {
                    let at_root = Glob::file(&format!("/{}", &rule[3..]));
                    (glob.picks_files() && glob.overlaps(forced)) || glob.overlaps(&at_root)
                }
                Forced::Folder | Forced::NameAtRoot => glob.overlaps(forced),
            });
            if let Some((rule, _, _)) = covered {
                found.push(issue(
                    format!("review.areas[{index}].paths[{path_index}]"),
                    format!("`{text}` covers `{rule}`, which is `platform` whatever the entries say: leave it to a `platform` entry, or anchor the glob with a leading `/`"),
                ));
            }
        }
    }
}

/// `botica.toml` fails to load when a glob of `generated` can select a path of `critical`, of
/// `platform` or of the forced list, since the size of a change would not count files that a
/// person has to read. A glob with no `/` is read at every depth, so `uv.lock` can select
/// `.github/workflows/uv.lock`: write `/uv.lock` for the file at the root.
fn generated_over_protected(found: &mut Vec<ConfigIssue>, config: &ReviewConfig) {
    let mut protected: Vec<(String, Glob)> = config
        .forced_platform_globs()
        .into_iter()
        .map(|glob| {
            let parsed = Forced::parsed(&glob);
            (glob, parsed)
        })
        .collect();
    for area in &config.areas {
        if matches!(area.name, AreaName::Critical | AreaName::Platform) {
            protected.extend(area.paths.iter().map(|g| (g.clone(), Glob::parse(g))));
        }
    }
    for (index, text) in config.generated.iter().enumerate() {
        let generated = Glob::file(text);
        if let Some((rule, _)) = protected.iter().find(|(_, glob)| generated.overlaps(glob)) {
            found.push(issue(
                format!("review.generated[{index}]"),
                format!("`{text}` can select a path of `{rule}`, which a person has to review: a change to it counts in full"),
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
