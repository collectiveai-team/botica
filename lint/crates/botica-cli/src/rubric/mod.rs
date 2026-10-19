//! The rubric of abide (`.abide/rubric.json`, `@coldtea/abide-schema` 0.0.5): the active judgment
//! rules of the project, each with its question, the line of `AGENTS.md` that lists it and the
//! files it applies to.

mod scope;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use botica_catalog::{Question, Rule, When};
use botica_config::Resolution;
use botica_core::{extensions_of, language_of, Diagnostic, Severity};
use serde::Serialize;
use serde_json::Value;

use crate::judgment;
use crate::project;

pub const DEFAULT_OUTPUT: &str = ".abide/rubric.json";
pub const DEFAULT_AGENTS_MD: &str = "AGENTS.md";

const VERSION: u32 = 1;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rubric<'a> {
    version: u32,
    compiled_at: String,
    compiled_by: String,
    manual: bool,
    sources: Vec<Source>,
    rules: Vec<RubricRule<'a>>,
}

#[derive(Serialize)]
struct Source {
    path: String,
}

#[derive(Serialize)]
struct RubricRule<'a> {
    id: &'a str,
    text: String,
    source: RuleSource,
    scope: Vec<String>,
    when: When,
    check: Check<'a>,
}

#[derive(Serialize)]
struct RuleSource {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
}

#[derive(Serialize)]
struct Check<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    question: &'a Question,
}

/// What the rubric is made from: the configuration of the project, the files of the project from
/// its root, how to read one, and the file of the block with the judgment rules.
pub struct Inputs<'a> {
    pub resolution: &'a Arc<Resolution>,
    pub files: &'a [String],
    pub read: &'a dyn Fn(&str) -> Option<String>,
    pub agents_md_path: &'a str,
    pub agents_md: &'a str,
}

/// The rubric of the project, or why the block of `AGENTS.md` cannot name its rules.
pub fn rubric<'a>(inputs: &Inputs<'a>, compiled_at: String) -> Result<Rubric<'a>, String> {
    let judged: Vec<&Rule> = inputs
        .resolution
        .entries()
        .iter()
        .filter(|entry| judgment::exported(inputs.resolution, &entry.rule, entry.active))
        .map(|entry| &entry.rule)
        .filter(|rule| when(rule).is_some_and(|when| when != When::PullRequest))
        .collect();
    let inside = inside(inputs, &judged);
    let mut rules = Vec::new();
    for rule in judged {
        let languages = languages(rule);
        let files: Vec<String> = inputs
            .files
            .iter()
            .filter(|file| of_languages(file, &languages))
            .cloned()
            .collect();
        let empty = BTreeSet::new();
        let extensions: Vec<&str> = languages
            .iter()
            .flat_map(|l| extensions_of(l))
            .copied()
            .collect();
        let scope = scope::globs(
            &files,
            inside.get(rule.id.as_str()).unwrap_or(&empty),
            &extensions,
        );
        if scope.is_empty() {
            continue;
        }
        let source = RuleSource {
            path: inputs.agents_md_path.to_owned(),
            line: Some(line_of(inputs, rule)?),
        };
        rules.push(rubric_rule(rule, source, scope));
    }
    let sources = vec![Source {
        path: inputs.agents_md_path.to_owned(),
    }];
    Ok(with_rules(sources, rules, compiled_at))
}

/// A rubric with `rule` alone, judged on the files of `scope`, for a test of the rule: its
/// source is `source`, with no line.
pub fn single<'a>(
    rule: &'a Rule,
    source: &str,
    scope: Vec<String>,
    compiled_at: String,
) -> Rubric<'a> {
    let source = RuleSource {
        path: source.to_owned(),
        line: None,
    };
    with_rules(
        Vec::new(),
        vec![rubric_rule(rule, source, scope)],
        compiled_at,
    )
}

fn with_rules<'a>(
    sources: Vec<Source>,
    rules: Vec<RubricRule<'a>>,
    compiled_at: String,
) -> Rubric<'a> {
    Rubric {
        version: VERSION,
        compiled_at,
        compiled_by: format!("botica {}", env!("CARGO_PKG_VERSION")),
        manual: true,
        sources,
        rules,
    }
}

fn rubric_rule<'a>(rule: &'a Rule, source: RuleSource, scope: Vec<String>) -> RubricRule<'a> {
    let implementation = &rule.implementations[0];
    RubricRule {
        id: &rule.id,
        text: rule.directive(),
        source,
        scope,
        when: implementation.when.unwrap_or(When::Edit),
        check: Check {
            kind: "model",
            question: implementation
                .question
                .as_ref()
                .expect("a judgment rule has a question"),
        },
    }
}

fn when(rule: &Rule) -> Option<When> {
    rule.implementations
        .first()
        .and_then(|implementation| implementation.when)
}

/// The languages of a rule, none when it applies to every language (`*`).
fn languages(rule: &Rule) -> Vec<&str> {
    let all = rule.languages();
    if all.contains(&"*") {
        Vec::new()
    } else {
        all
    }
}

fn of_languages(file: &str, languages: &[&str]) -> bool {
    languages.is_empty() || language_of(file).is_some_and(|language| languages.contains(&language))
}

/// For each rule, the files it applies to: those of its languages where the configuration applies
/// it and no `ignore-file` comment silences it, as `botica lint --judged` keeps a finding.
fn inside(inputs: &Inputs<'_>, rules: &[&Rule]) -> BTreeMap<String, BTreeSet<String>> {
    let mut by_path: BTreeMap<String, Vec<Diagnostic>> = BTreeMap::new();
    for file in inputs.files {
        for rule in rules {
            if of_languages(file, &languages(rule)) {
                by_path
                    .entry(file.clone())
                    .or_default()
                    .push(candidate(file, &rule.id));
            }
        }
    }
    let kept = project::keep_judged(inputs.resolution, None, by_path, inputs.read);
    let mut inside: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for found in kept {
        inside.entry(found.rule).or_default().insert(found.path);
    }
    inside
}

/// A finding of `rule` about the whole of `path`.
fn candidate(path: &str, rule: &str) -> Diagnostic {
    Diagnostic {
        path: path.to_owned(),
        rule: rule.to_owned(),
        severity: Severity::Warning,
        start: 0,
        end: 0,
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 1,
        file_level: true,
        message: String::new(),
    }
}

/// The line of the block of `AGENTS.md` that lists `rule`, from 1.
fn line_of(inputs: &Inputs<'_>, rule: &Rule) -> Result<usize, String> {
    let start = format!("- {} `{}`:", rule.code, rule.id);
    inputs
        .agents_md
        .lines()
        .position(|line| line.starts_with(&start))
        .map(|at| at + 1)
        .ok_or_else(|| {
            format!(
                "`{0}` has no line for `{1}`: run `botica rules export --format agents-md --output \
                 {0}` first",
                inputs.agents_md_path, rule.id
            )
        })
}

pub fn render(rubric: &Rubric<'_>) -> String {
    let mut text = serde_json::to_string_pretty(rubric).expect("the rubric is JSON");
    text.push('\n');
    text
}

/// Whether the rubric in `existing` has the sources and the rules of `rubric`. What
/// `abide rubric validate` adds when it rewrites the file does not count: the `sha` of a source
/// and a `status` of `active` written out; nor do `compiledAt` and `compiledBy`.
pub fn unchanged(existing: &str, rubric: &Rubric<'_>) -> bool {
    let Ok(mut existing) = serde_json::from_str::<Value>(existing) else {
        return false;
    };
    for source in array_mut(&mut existing, "sources") {
        if let Some(source) = source.as_object_mut() {
            source.remove("sha");
        }
    }
    for rule in array_mut(&mut existing, "rules") {
        if let Some(rule) = rule.as_object_mut() {
            if rule.get("status").and_then(Value::as_str) == Some("active") {
                rule.remove("status");
            }
        }
    }
    let wanted = serde_json::to_value(rubric).expect("the rubric is JSON");
    ["sources", "rules"]
        .iter()
        .all(|key| existing.get(key) == wanted.get(key))
}

fn array_mut<'v>(value: &'v mut Value, key: &str) -> impl Iterator<Item = &'v mut Value> {
    value
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
}

/// `seconds` since 1970 as a time of RFC 3339 in UTC, as `compiledAt` writes it.
pub fn rfc3339(seconds: u64) -> String {
    let days = seconds / 86_400;
    let rest = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The date of a day counted from 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests;
