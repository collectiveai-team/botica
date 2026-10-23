//! The selection of rules of ruff: which codes the settings of a project switch on or off.

use std::collections::HashMap;
use std::sync::OnceLock;

use toml::{Table, Value};

/// The keys of a fragment that name codes the rule needs selected: the keys of `[lint]` and their
/// old place at the top of the settings, which ruff 0.17 still reads with a warning.
const SELECTING: [&str; 2] = ["select", "extend-select"];
const IGNORING: [&str; 2] = ["ignore", "extend-ignore"];

/// The rules of the version of ruff that the recipes pin, one per line.
const RULES: &str = include_str!("ruff_rules.tsv");

/// A rule of ruff: its code, the prefix of its linter (`A`, `ASYNC`, `PL`, and none for the `E`
/// and `W` of pycodestyle), and whether it is in preview.
struct Known {
    code: &'static str,
    linter: &'static str,
    preview: bool,
}

/// The rules of the table, in its order, and the place of each code in it.
fn known_rules() -> &'static (Vec<Known>, HashMap<&'static str, usize>) {
    static KNOWN: OnceLock<(Vec<Known>, HashMap<&'static str, usize>)> = OnceLock::new();
    KNOWN.get_or_init(|| {
        let rules: Vec<Known> = RULES
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
            .filter_map(|line| {
                let mut fields = line.split('\t');
                Some(Known {
                    code: fields.next()?,
                    linter: fields.next()?,
                    preview: fields.next()? == "preview",
                })
            })
            .collect();
        let places = rules
            .iter()
            .enumerate()
            .map(|(place, rule)| (rule.code, place))
            .collect();
        (rules, places)
    })
}

fn known(code: &str) -> Option<&'static Known> {
    let (rules, places) = known_rules();
    places.get(code).map(|place| &rules[*place])
}

/// Takes out of `fragment` the codes that it selects, and gives them back. A `[lint]` left empty
/// goes too, so that the rest of the fragment does not ask for one.
pub fn needed(fragment: &mut Table) -> Vec<String> {
    let mut codes = Vec::new();
    for key in SELECTING {
        codes.extend(take(fragment, key));
        if let Some(Value::Table(lint)) = fragment.get_mut("lint") {
            codes.extend(take(lint, key));
        }
    }
    if fragment
        .get("lint")
        .and_then(Value::as_table)
        .is_some_and(Table::is_empty)
    {
        fragment.remove("lint");
    }
    codes
}

fn take(table: &mut Table, key: &str) -> Vec<String> {
    match table.remove(key) {
        Some(Value::Array(items)) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// The value of a key of `[lint]`, or of its old place at the top of the settings.
pub fn flag(settings: &Table, key: &str) -> bool {
    let lint = settings.get("lint").and_then(Value::as_table);
    [lint, Some(settings)]
        .into_iter()
        .flatten()
        .find_map(|table| table.get(key)?.as_bool())
        .unwrap_or(false)
}

/// What the settings of a project do with a selector of the fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every rule it names is on.
    On,
    /// A selector of `ignore` or `extend-ignore` switches a rule it names off: the project decided
    /// it.
    Off { by: String },
    /// A rule it names is neither selected nor ignored.
    Missing,
}

/// The selectors of the settings of a project, as ruff applies them.
pub struct Selection<'t> {
    selects: Vec<&'t str>,
    ignores: Vec<&'t str>,
    /// `explicit-preview-rules`: a rule in preview is on only when a selector is its code.
    explicit: bool,
}

impl<'t> Selection<'t> {
    /// The selection of `layers`, the settings of ruff with the file they extend first. The layer
    /// of highest priority that has `select` starts the selection again, as in ruff, so the layers
    /// below it do not count. Without any `select`, ruff applies its default rules, which change
    /// from one version to the next, and only `extend-select` counts.
    pub fn of(layers: &'t [Table], explicit: bool) -> Self {
        let start = layers
            .iter()
            .rposition(|layer| !selectors(layer, "select").is_empty())
            .unwrap_or(0);
        let mut selects = Vec::new();
        let mut ignores = Vec::new();
        for layer in &layers[start..] {
            for key in SELECTING {
                selects.extend(selectors(layer, key));
            }
            for key in IGNORING {
                ignores.extend(selectors(layer, key));
            }
        }
        Self {
            selects,
            ignores,
            explicit,
        }
    }

    /// What the selection does with each rule that `selector` of the fragment names. A family
    /// names its stable rules, and also those in preview when `preview` says the fragment turns
    /// them on through a family; a code names itself.
    pub fn verdict(&self, selector: &str, preview: bool) -> Verdict {
        let codes = expand(selector, preview);
        let verdicts: Vec<Verdict> = codes.iter().map(|code| self.code(code)).collect();
        if verdicts.contains(&Verdict::Missing) {
            return Verdict::Missing;
        }
        verdicts
            .into_iter()
            .find(|verdict| *verdict != Verdict::On)
            .unwrap_or(Verdict::On)
    }

    /// The most specific selector that names `code` decides, as in ruff, and `ignore` wins a tie.
    fn code(&self, code: &str) -> Verdict {
        let in_preview = known(code).is_some_and(|rule| rule.preview);
        let selected = self
            .selects
            .iter()
            .filter(|selector| covers(selector, code))
            .filter(|selector| !(in_preview && self.explicit && **selector != code))
            .map(|selector| specificity(selector))
            .max();
        let ignored = self
            .ignores
            .iter()
            .filter(|selector| covers(selector, code))
            .max_by_key(|selector| specificity(selector));
        match (selected, ignored) {
            (Some(on), Some(off)) if specificity(off) < on => Verdict::On,
            (_, Some(off)) => Verdict::Off {
                by: (*off).to_owned(),
            },
            (Some(_), None) => Verdict::On,
            (None, None) => Verdict::Missing,
        }
    }
}

/// The codes that `selector` names: itself when it is a code or ruff does not know it, or else
/// the rules of the table it takes in.
fn expand(selector: &str, preview: bool) -> Vec<&str> {
    if known(selector).is_some() {
        return vec![selector];
    }
    let codes: Vec<&str> = known_rules()
        .0
        .iter()
        .filter(|rule| (preview || !rule.preview) && covers(selector, rule.code))
        .map(|rule| rule.code)
        .collect();
    if codes.is_empty() {
        vec![selector]
    } else {
        codes
    }
}

fn specificity(selector: &str) -> usize {
    if selector == "ALL" {
        0
    } else {
        selector.len()
    }
}

fn selectors<'t>(layer: &'t Table, key: &str) -> Vec<&'t str> {
    let lint = layer.get("lint").and_then(Value::as_table);
    [Some(layer), lint]
        .into_iter()
        .flatten()
        .filter_map(|table| table.get(key)?.as_array())
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

/// Whether the selector of ruff `selector` takes in every rule that `code` names: `A` takes in
/// `A005` and not `ASYNC100`, `PL` takes in `PLR0124`, `C` takes in `C4` and `C90`, `ALL` takes in
/// everything. A selector that names part of `code` does not: `A005` does not take in `A`.
pub fn covers(selector: &str, code: &str) -> bool {
    if selector == "ALL" {
        return true;
    }
    if let Some(rule) = known(code) {
        let legacy = matches!(
            (selector, rule.linter),
            ("C", "C4" | "C90") | ("T", "T10" | "T20")
        );
        return code.starts_with(selector) && (selector.len() >= rule.linter.len() || legacy);
    }
    let (linter, digits) = split(selector);
    let (code_linter, code_digits) = split(code);
    let same_linter =
        linter == code_linter || (linter == "PL" && digits.is_empty() && is_pylint(code_linter));
    same_linter && code_digits.starts_with(digits)
}

/// The letters of a code, which name its linter, and the digits after them.
fn split(code: &str) -> (&str, &str) {
    code.split_at(
        code.find(|letter: char| letter.is_ascii_digit())
            .unwrap_or(code.len()),
    )
}

fn is_pylint(letters: &str) -> bool {
    matches!(letters, "PLC" | "PLE" | "PLR" | "PLW")
}
