//! `external-config`: botica checks that the tool of an active external rule is set up. It reads
//! the settings of the tool from the files the tool reads them from, and finds there the fragment
//! of configuration that the rule declares. A key that the settings lack is a finding; a key that
//! they set to a value weaker than the fragment is the decision of the project, and `weakened`
//! gives it to the rules that report it.

mod duration;
mod in_use;
mod pyrefly;
mod ruff;
mod tools;

use botica_catalog::{Format, Kind, Rule};
use toml::{Table, Value};

use crate::diagnostic::{Diagnostic, Severity};
pub use in_use::in_use;
use tools::{Place, Reading, Span, Tool};

pub const EXTERNAL_CONFIG: &str = "external-config";

/// The longest chain of files that extend one another that botica follows.
const MAX_EXTENDS: usize = 8;

/// A fragment of configuration that a rule needs in a file of the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement<'a> {
    pub rule: &'a str,
    pub tool: &'a str,
    /// The file, from the root of the project.
    pub target: &'a str,
    pub fragment: &'a str,
}

impl<'a> Requirement<'a> {
    /// The fragments that the external implementations of `rule` declare.
    pub fn of(rule: &'a Rule) -> Vec<Self> {
        Self::of_used(rule, None)
    }

    /// The fragments of the external implementations of `rule` that the project uses, its files
    /// from the root being `files` (`in_use`); every fragment when `files` is `None`.
    pub fn of_used(rule: &'a Rule, files: Option<&[String]>) -> Vec<Self> {
        rule.implementations
            .iter()
            .filter(|implementation| implementation.kind == Kind::External)
            .filter(|implementation| {
                files.is_none_or(|files| {
                    in_use(
                        &implementation.languages,
                        &implementation.applies_when,
                        files,
                    )
                })
            })
            .filter_map(|implementation| {
                Some(Self {
                    rule: &rule.id,
                    tool: implementation.tool.as_deref()?,
                    target: implementation.target()?,
                    fragment: implementation.source.as_deref()?,
                })
            })
            .collect()
    }

    fn label(&self) -> String {
        format!("`{}`", self.rule)
    }
}

/// A value that the settings of a tool set on purpose, and that holds less than the fragment of a
/// rule asks: a shorter cooldown, `preset = "basic"` where the rule asks `strict`, a code of ruff
/// in `ignore`. `external-config` does not report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weakening {
    pub rule: String,
    pub tool: String,
    /// The file the tool reads, from the root of the project.
    pub file: String,
    /// The keys from the table of the tool to the value, joined with dots: `exclude-newer`,
    /// `errors.explicit-any`, and `lint.select` for a code of ruff.
    pub key: String,
    /// What the fragment asks, as TOML: `"7 days"`, `"strict"`, `"DOC102"`.
    pub wanted: String,
    /// What the settings hold, as TOML: `"3 days"`, `"basic"`, `ignore = ["DOC"]`.
    pub found: String,
}

/// One finding for each requirement whose tool has no file of settings, whose file cannot be read
/// in its format, or whose settings lack a key of the fragment. `read` gives the text of a file
/// from the root of the project, `None` when it does not exist.
pub fn check(
    requirements: &[Requirement<'_>],
    read: impl Fn(&str) -> Option<String>,
    severity: Severity,
) -> Vec<Diagnostic> {
    requirements
        .iter()
        .filter_map(|requirement| finding(requirement, &read))
        .map(|(path, message)| Diagnostic::file_level(&path, EXTERNAL_CONFIG, severity, message))
        .collect()
}

/// The values of the settings that hold less than the fragments of `requirements`, in the order
/// of the requirements and of the keys of each fragment.
pub fn weakened(
    requirements: &[Requirement<'_>],
    read: impl Fn(&str) -> Option<String>,
) -> Vec<Weakening> {
    requirements
        .iter()
        .flat_map(|requirement| {
            let Ok(Some(evaluation)) = evaluate(requirement, &read) else {
                return Vec::new();
            };
            evaluation
                .tally
                .weaker
                .into_iter()
                .map(|weaker| Weakening {
                    rule: requirement.rule.to_owned(),
                    tool: requirement.tool.to_owned(),
                    file: evaluation.file.clone(),
                    key: weaker.key,
                    wanted: weaker.wanted,
                    found: weaker.found,
                })
                .collect()
        })
        .collect()
}

/// The file to report and the message, when the settings of the tool lack the fragment.
fn finding(
    requirement: &Requirement<'_>,
    read: &impl Fn(&str) -> Option<String>,
) -> Option<(String, String)> {
    let (target, name, label) = (requirement.target, requirement.tool, requirement.label());
    let explain = format!("`botica explain {}`", requirement.rule);
    let lacks = |file: &str| {
        format!("{file} lacks the configuration of {name} that {label} needs: {explain} shows it")
    };
    match evaluate(requirement, read) {
        Err(broken) => Some((
            broken.file.clone(),
            format!(
                "{} is not {}, so botica cannot find the configuration of {name} that {label} \
                 needs: {}",
                broken.file,
                broken.format.name(),
                broken.error
            ),
        )),
        Ok(Some(evaluation)) if evaluation.tally.missing => {
            Some((evaluation.file.clone(), lacks(&evaluation.file)))
        }
        Ok(Some(_)) => None,
        Ok(None) if read(target).is_some() => Some((target.to_owned(), lacks(target))),
        Ok(None) => {
            let alone = tools::alone(target);
            let tool = Tool::for_target(name, target, &alone);
            let mut files: Vec<&str> = tool.places.iter().map(|place| place.file).collect();
            files.dedup();
            let message = if files.len() == 1 {
                format!(
                    "{target} does not exist, and {name} needs it for {label}: {explain} shows \
                     what goes in it"
                )
            } else {
                format!(
                    "no file of settings of {name} exists ({}), and {label} needs one: {explain} \
                     shows what goes in it",
                    files.join(", ")
                )
            };
            Some((target.to_owned(), message))
        }
    }
}

/// How the settings of the tool of a requirement hold its fragment.
struct Evaluation {
    /// The file the tool reads.
    file: String,
    tally: Tally,
}

/// The settings of the tool of `requirement` measured against its fragment; `None` when the tool
/// has no file of settings.
fn evaluate(
    requirement: &Requirement<'_>,
    read: &impl Fn(&str) -> Option<String>,
) -> Result<Option<Evaluation>, Broken> {
    let target = requirement.target;
    let alone = tools::alone(target);
    let tool = Tool::for_target(requirement.tool, target, &alone);
    let fragment = Format::of(target)
        .and_then(|format| format.parse(requirement.fragment).ok())
        .unwrap_or_default();
    let place = tool.place_of(target, &fragment);
    let wanted = lookup(&fragment, place.table).cloned().unwrap_or_default();
    let Some(settings) = settings(&tool, read)? else {
        return Ok(None);
    };
    let tally = measure(&tool, &settings, wanted);
    Ok(Some(Evaluation {
        file: settings.file,
        tally,
    }))
}

/// The settings of a tool as it reads them: the file it reads and its table there, after the
/// tables of the files it extends.
struct Settings {
    file: String,
    /// The file that is extended first, the file the tool reads last.
    layers: Vec<Table>,
    text: bool,
}

/// A file of settings that botica cannot read in its format.
struct Broken {
    file: String,
    format: Format,
    error: String,
}

/// The settings that the first place of `tool` that counts holds, `None` when no place counts.
fn settings(
    tool: &Tool<'_>,
    read: &impl Fn(&str) -> Option<String>,
) -> Result<Option<Settings>, Broken> {
    for place in tool.places {
        let Some(table) = table_at(tool, place, read)? else {
            continue;
        };
        let mut layers = vec![table];
        let mut from = place.file.to_owned();
        while layers.len() <= MAX_EXTENDS {
            let next = tool
                .extends
                .and_then(|key| layers.last()?.get(key)?.as_str())
                .and_then(|path| joined(&from, path));
            let Some(next) = next else {
                break;
            };
            let base = Place {
                file: &next,
                table: tool
                    .places
                    .iter()
                    .find(|other| name_of(other.file) == name_of(&next))
                    .map_or(&[][..], |other| other.table),
                always: true,
                text: place.text,
            };
            let Some(table) = table_at(tool, &base, read)? else {
                break;
            };
            layers.push(table);
            from = next;
        }
        layers.reverse();
        return Ok(Some(Settings {
            file: place.file.to_owned(),
            layers,
            text: place.text,
        }));
    }
    Ok(None)
}

/// The table of `place`, `None` when its file does not exist or the tool passes over it.
fn table_at(
    tool: &Tool<'_>,
    place: &Place<'_>,
    read: &impl Fn(&str) -> Option<String>,
) -> Result<Option<Table>, Broken> {
    let Some(text) = read(place.file) else {
        return Ok(None);
    };
    let format = Format::of(place.file).unwrap_or(Format::Toml);
    let whole = format.parse(&text).map_err(|error| Broken {
        file: place.file.to_owned(),
        format,
        error,
    })?;
    let mut table = match lookup(&whole, place.table) {
        Some(table) => table.clone(),
        None if place.always => Table::new(),
        None => return Ok(None),
    };
    let mut had_nested = false;
    for key in tool.nested(place) {
        had_nested |= table.remove(key).is_some();
    }
    if had_nested && table.is_empty() && !place.always {
        return Ok(None);
    }
    Ok(Some(table))
}

/// The path from the root of the project of `path`, written in the file `from`. `None` when it
/// is absolute or leaves the project.
fn joined(from: &str, path: &str) -> Option<String> {
    if path.starts_with('/') || path.contains(':') {
        return None;
    }
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            _ => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The table at `path` inside `table`; `table` itself for an empty path.
fn lookup<'t>(table: &'t Table, path: &[&str]) -> Option<&'t Table> {
    path.iter()
        .try_fold(table, |table, key| table.get(*key)?.as_table())
}

/// What the settings lack and what they hold weaker than the fragment.
#[derive(Default)]
struct Tally {
    /// A key of the fragment, or an item of one of its lists, that the settings do not set.
    missing: bool,
    weaker: Vec<Weaker>,
}

struct Weaker {
    key: String,
    wanted: String,
    found: String,
}

impl Tally {
    fn weaker(&mut self, path: &[String], wanted: &Value, found: String) {
        self.weaker.push(Weaker {
            key: path.join("."),
            wanted: wanted.to_string(),
            found,
        });
    }
}

fn measure(tool: &Tool<'_>, settings: &Settings, mut wanted: Table) -> Tally {
    let merged = merge(&settings.layers);
    let compare = Compare {
        text: settings.text,
        spans: tool.spans,
    };
    let mut tally = Tally::default();
    match tool.reading {
        Reading::Plain => compare.table(&mut Vec::new(), &merged, &wanted, &mut tally),
        Reading::Ruff => {
            // A family takes in its rules in preview when the fragment turns preview on and does
            // not ask for each of them by its code.
            let preview =
                ruff::flag(&wanted, "preview") && !ruff::flag(&wanted, "explicit-preview-rules");
            let codes = ruff::needed(&mut wanted);
            let selection = ruff::Selection::of(
                &settings.layers,
                ruff::flag(&merged, "explicit-preview-rules"),
            );
            let path = vec!["lint".to_owned(), "select".to_owned()];
            for code in codes {
                match selection.verdict(&code, preview) {
                    ruff::Verdict::On => {}
                    ruff::Verdict::Off { by } => {
                        tally.weaker(&path, &Value::String(code), format!("ignore = [\"{by}\"]"));
                    }
                    ruff::Verdict::Missing => tally.missing = true,
                }
            }
            compare.table(&mut Vec::new(), &merged, &wanted, &mut tally);
        }
        Reading::Pyrefly => pyrefly(&merged, wanted, &compare, &mut tally),
    }
    tally
}

/// The tables of `layers` merged key by key, a later layer over an earlier one.
fn merge(layers: &[Table]) -> Table {
    let mut merged = Table::new();
    for layer in layers {
        merge_into(&mut merged, layer);
    }
    merged
}

fn merge_into(into: &mut Table, layer: &Table) {
    for (key, value) in layer {
        match (into.get_mut(key), value) {
            (Some(Value::Table(table)), Value::Table(over)) => merge_into(table, over),
            _ => {
                into.insert(key.clone(), value.clone());
            }
        }
    }
}

/// A preset of pyrefly holds the one the fragment names when it checks at least as much: `all`
/// checks what `strict` checks, and `strict` what `default` checks. A kind of error that the
/// fragment sets holds when the project sets it at least as high, or when the preset is `all`.
fn pyrefly(merged: &Table, mut wanted: Table, compare: &Compare<'_>, tally: &mut Tally) {
    const LADDER: [&str; 3] = ["default", "strict", "all"];
    let preset = merged.get("preset");
    let rank = |name: &str| LADDER.iter().position(|step| *step == name);
    match (wanted.remove("preset"), preset) {
        (None, _) => {}
        (Some(_), None) => tally.missing = true,
        (Some(Value::String(needed)), Some(Value::String(have)))
            if *have == needed
                || matches!((rank(have), rank(&needed)), (Some(h), Some(n)) if h >= n) => {}
        (Some(needed), Some(have)) => {
            tally.weaker(&["preset".to_owned()], &needed, have.to_string());
        }
    }
    let all = preset.and_then(Value::as_str) == Some("all");
    if let Some(needed) = wanted.remove("errors") {
        let set = merged.get("errors").and_then(Value::as_table);
        let kinds = needed.as_table().cloned().unwrap_or_default();
        for (kind, need) in &kinds {
            match set.and_then(|set| set.get(kind)) {
                _ if all => {}
                None => tally.missing = true,
                Some(have) if pyrefly::severity(kind, have) >= pyrefly::severity(kind, need) => {}
                Some(have) => {
                    tally.weaker(&["errors".to_owned(), kind.clone()], need, have.to_string())
                }
            }
        }
    }
    compare.table(&mut Vec::new(), merged, &wanted, tally);
}

/// How a value of the settings holds a value of the fragment.
struct Compare<'t> {
    /// The values of the settings are text, so `"true"` holds `true` and `"-ra --strict-markers"`
    /// holds `["--strict-markers"]`.
    text: bool,
    spans: &'t [Span],
}

impl Compare<'_> {
    /// Tallies each key of `fragment` that `table` lacks, and each that it sets to a value that
    /// does not hold the value of the fragment. `path` is the place of `table` in the settings.
    fn table(&self, path: &mut Vec<String>, table: &Table, fragment: &Table, tally: &mut Tally) {
        for (key, wanted) in fragment {
            path.push(key.clone());
            match table.get(key) {
                Some(value) => self.value(path, value, wanted, tally),
                None => tally.missing = true,
            }
            path.pop();
        }
    }

    /// A table holds the keys it names and an array one item for each item of the fragment; an
    /// item it lacks is missing. A time span holds a span that is not shorter, and any other value
    /// an equal one; another value there is weaker.
    fn value(&self, path: &mut Vec<String>, value: &Value, wanted: &Value, tally: &mut Tally) {
        let span = self.spans.iter().find(|span| {
            span.key.len() == path.len() && span.key.iter().zip(path.iter()).all(|(a, b)| a == b)
        });
        let held = if let Some(span) = span {
            match (
                duration::seconds(value, span.unit),
                duration::seconds(wanted, span.unit),
            ) {
                (Some(have), Some(need)) => have >= need,
                _ => self.same(value, wanted),
            }
        } else {
            match (value, wanted) {
                (Value::Table(table), Value::Table(fragment)) => {
                    return self.table(path, table, fragment, tally);
                }
                (Value::Array(items), Value::Array(needed)) => {
                    tally.missing |= !needed
                        .iter()
                        .all(|need| items.iter().any(|item| self.holds(path, item, need)));
                    return;
                }
                (Value::String(text), Value::Array(needed)) if self.text => {
                    let items = words(text);
                    tally.missing |= !needed.iter().all(|need| {
                        as_text(need).is_some_and(|need| items.contains(&need.as_str()))
                    });
                    return;
                }
                _ => self.same(value, wanted),
            }
        };
        if !held {
            tally.weaker(path, wanted, value.to_string());
        }
    }

    /// Whether `value` holds `wanted` with nothing missing and nothing weaker, as an item of a
    /// list has to: `[[repos]]` holds the hook of the fragment when one of its items does.
    fn holds(&self, path: &mut Vec<String>, value: &Value, wanted: &Value) -> bool {
        let mut tally = Tally::default();
        self.value(path, value, wanted, &mut tally);
        !tally.missing && tally.weaker.is_empty()
    }

    /// Whether `value` is `wanted`; as text, case aside, when the settings are text.
    fn same(&self, value: &Value, wanted: &Value) -> bool {
        match value {
            Value::String(text) if self.text => {
                as_text(wanted).is_some_and(|need| need.eq_ignore_ascii_case(text.trim()))
            }
            _ => value == wanted,
        }
    }
}

fn as_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Integer(number) => Some(number.to_string()),
        Value::Float(number) => Some(number.to_string()),
        Value::Boolean(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// The items of a list written as text: one for each line when it has several, as pytest writes
/// `markers`, or else one for each word, as in `addopts`.
fn words(text: &str) -> Vec<&str> {
    if text.trim().contains('\n') {
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    } else {
        text.split_whitespace().collect()
    }
}
