use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use botica_config::{FileRule, Resolution};
use botica_core::postprocess::Verdict;
use botica_core::registry::IO_ERROR;
use botica_core::{language_of, Exclusion, NativePlanner, Severity};

/// A path as the configuration sees it: from the project root, not from where botica runs.
fn from_root<'p>(prefix: Option<&str>, path: &'p str) -> Cow<'p, str> {
    match prefix {
        Some(prefix) => Cow::Owned(format!("{prefix}/{path}")),
        None => Cow::Borrowed(path),
    }
}

pub fn planner(
    resolution: Arc<Resolution>,
    index: HashMap<String, usize>,
    prefix: Option<String>,
) -> NativePlanner {
    Box::new(move |path| {
        resolution
            .rules_for(&from_root(prefix.as_deref(), path), language_of(path))
            .iter()
            .filter_map(|file_rule| index.get(file_rule.rule.id.as_str()).copied())
            .collect()
    })
}

/// What `extend-exclude` leaves out of the walk, or `None` when it names nothing.
pub fn exclusion(resolution: Arc<Resolution>, prefix: Option<String>) -> Option<Exclusion> {
    if !resolution.has_exclusions() {
        return None;
    }
    let (for_entries, for_paths) = (Arc::clone(&resolution), resolution);
    let (entry_prefix, path_prefix) = (prefix.clone(), prefix);
    Some(Exclusion {
        entry: Box::new(move |path, is_dir| {
            for_entries.excludes_entry(&from_root(entry_prefix.as_deref(), path), is_dir)
        }),
        path: Box::new(move |path| {
            for_paths.excludes_path(&from_root(path_prefix.as_deref(), path))
        }),
    })
}

/// The rules that apply at one path, each with the severity the configuration names for it.
type Named<'r> = Vec<(&'r str, Option<Severity>)>;

/// What the configuration says about a rule at a path, for the post-process. A rule of the
/// catalog that does not apply at the path is dropped; one that has a severity named for it
/// there gets it; the rules the catalog does not know (ast-grep's, extensions') are left alone.
pub fn verdicts<'r>(
    resolution: &'r Resolution,
    prefix: Option<&'r str>,
) -> impl FnMut(&str, &str) -> Verdict + 'r {
    let mut current: Option<(String, Named<'r>)> = None;
    move |path, rule| {
        let Some(entry) = resolution.entry(rule) else {
            return Verdict::Keep;
        };
        let at_root = from_root(prefix, path);
        if entry.rule.id == IO_ERROR {
            return verdict_in(&resolution.rules_for_path(&at_root), &entry.rule.id);
        }
        if current.as_ref().is_none_or(|(known, _)| known != path) {
            let rules = resolution.rules_for(&at_root, language_of(path));
            let found = rules
                .iter()
                .map(|file_rule| {
                    (
                        file_rule.rule.id.as_str(),
                        file_rule.configured.map(Severity::from),
                    )
                })
                .collect();
            current = Some((path.to_owned(), found));
        }
        let (_, found) = current.as_ref().expect("filled just above");
        match found.iter().find(|(id, _)| *id == entry.rule.id) {
            Some((_, Some(level))) => Verdict::Report(*level),
            Some((_, None)) => Verdict::Keep,
            None => Verdict::Drop,
        }
    }
}

fn verdict_in(rules: &[FileRule<'_>], id: &str) -> Verdict {
    match rules.iter().find(|file_rule| file_rule.rule.id == id) {
        Some(FileRule {
            configured: Some(level),
            ..
        }) => Verdict::Report(Severity::from(*level)),
        Some(_) => Verdict::Keep,
        None => Verdict::Drop,
    }
}
