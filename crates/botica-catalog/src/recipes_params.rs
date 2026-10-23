//! The parameters of a recipe: a value that its templates write as `{{ param.<name> }}`, with a
//! default that the project changes in `[recipes.params]` of its configuration or with
//! `botica recipe install --set <name>=<value>`. A template does no arithmetic and has no
//! condition: a parameter of the kind `days` also gives its value in minutes and in seconds, and one
//! of the kind `version` gives `@<value>`, or nothing when the value is empty. `botica-version` is
//! declared by botica itself, with the release of the binary that runs as its default.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer};

use crate::recipes::Recipe;
use crate::validate::is_kebab_case;

const OPEN: &str = "{{ param.";
const CLOSE: &str = " }}";

/// The parameter that every chain has: the release of botica that the templates run, by default
/// the one of the binary, so the files ask for the rules that it knows. It takes the place of the
/// rewrite of `botica@X.Y.Z` and of the `rev` of the mirror that `install` made before.
pub const BOTICA_VERSION: &str = "botica-version";

/// What declares `BOTICA_VERSION`, in the place of a recipe.
pub const BUILT_IN: &str = "botica";

/// The parameters that botica declares, before those of the recipes.
pub(crate) fn built_in() -> Vec<ResolvedParam> {
    let version = env!("CARGO_PKG_VERSION");
    vec![ResolvedParam {
        name: BOTICA_VERSION.to_owned(),
        spec: ParamSpec {
            description: "The release of botica that the hooks and the workflows run".to_owned(),
            default: version.to_owned(),
            kind: ParamKind::Version,
            choices: Vec::new(),
        },
        value: version.to_owned(),
        from: BUILT_IN.to_owned(),
    }]
}

/// What a value of a parameter is, and the views of it that a template may write.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParamKind {
    /// One line of text: `{{ param.<name> }}`.
    #[default]
    Text,
    /// A whole number of days: `{{ param.<name> }}`, `{{ param.<name>.minutes }}` and
    /// `{{ param.<name>.seconds }}`.
    Days,
    /// The version of a tool, or empty for none: `{{ param.<name> }}`, which an empty value cannot
    /// fill, and `{{ param.<name>.at }}`, which is `@<value>` or nothing.
    Version,
}

impl ParamKind {
    fn views(self) -> &'static [&'static str] {
        match self {
            Self::Text => &[],
            Self::Days => &["minutes", "seconds"],
            Self::Version => &["at"],
        }
    }

    /// The name of the kind, as a recipe writes it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Days => "days",
            Self::Version => "version",
        }
    }

    /// Why `value` is not a value of this kind, if it is not.
    fn problem(self, value: &str) -> Option<&'static str> {
        if value.contains(['\n', '\r']) || value.contains("{{") {
            return Some("is not one line without `{{`");
        }
        match self {
            Self::Text => None,
            Self::Days => (value.parse::<u32>().is_err() || value.starts_with('+'))
                .then_some("is not a whole number of days"),
            Self::Version => (!value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".+-_~^*<>=!,".contains(c)))
            .then_some("is not a version: letters, digits and . + - _ ~ ^ * < > = ! ,"),
        }
    }
}

/// `[params.<name>]` of a recipe.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamSpec {
    /// One line for `botica recipe show`.
    pub description: String,
    #[serde(deserialize_with = "text_or_number")]
    pub default: String,
    #[serde(default)]
    pub kind: ParamKind,
    /// The values it may take; any value of its kind when it is empty.
    #[serde(default)]
    pub choices: Vec<String>,
}

/// A value of TOML that is a string or a whole number, as text.
pub(crate) fn text_or_number<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Value {
        Text(String),
        Number(u32),
    }
    Ok(match Value::deserialize(deserializer)? {
        Value::Text(text) => text,
        Value::Number(number) => number.to_string(),
    })
}

/// A parameter with the value it takes, and the recipe that declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedParam {
    pub name: String,
    pub spec: ParamSpec,
    pub value: String,
    pub from: String,
}

impl ResolvedParam {
    /// Why `value` cannot be the value of this parameter, if it cannot.
    fn value_problem(&self, value: &str) -> Option<String> {
        if let Some(problem) = self.spec.kind.problem(value) {
            return Some(format!(
                "`{value}` {problem}, so it is not a value of the parameter `{}`",
                self.name
            ));
        }
        (!self.spec.choices.is_empty() && !self.spec.choices.iter().any(|c| c == value)).then(
            || {
                format!(
                    "`{value}` is not a value of the parameter `{}` ({})",
                    self.name,
                    self.spec.choices.join(", ")
                )
            },
        )
    }

    /// The text of the view `view` of the value: the value itself when `view` is `None`.
    fn view(&self, view: Option<&str>) -> Result<String, String> {
        let value = &self.value;
        match (self.spec.kind, view) {
            (ParamKind::Version, None) if value.is_empty() => Err(format!(
                "the parameter `{}` is empty, and the template needs a version",
                self.name
            )),
            (_, None) => Ok(value.clone()),
            (ParamKind::Days, Some(unit)) => {
                let days: u64 = value.parse().unwrap_or_default();
                Ok(match unit {
                    "minutes" => days * 24 * 60,
                    _ => days * 24 * 60 * 60,
                }
                .to_string())
            }
            (ParamKind::Version, Some(_)) if value.is_empty() => Ok(String::new()),
            (ParamKind::Version, Some(_)) => Ok(format!("@{value}")),
            (ParamKind::Text, Some(view)) => Err(format!("a text has no view `{view}`")),
        }
    }
}

/// What is wrong with the parameters that one recipe declares, one message for each problem.
pub(crate) fn param_problems(params: &BTreeMap<String, ParamSpec>) -> Vec<String> {
    let mut problems = Vec::new();
    for (name, spec) in params {
        if !is_kebab_case(name) {
            problems.push(format!("the parameter `{name}` is not a kebab-case name"));
        }
        if name == BOTICA_VERSION {
            problems.push(format!(
                "botica declares the parameter `{name}`; a recipe does not"
            ));
        }
        if spec.description.trim().is_empty() {
            problems.push(format!("the parameter `{name}` has no description"));
        }
        problems.extend(value_problems(name, spec));
        problems.extend(choice_problems(name, spec));
    }
    problems
}

/// The default and the choices of the parameter `name` that its kind does not accept.
fn value_problems(name: &str, spec: &ParamSpec) -> Vec<String> {
    std::iter::once(&spec.default)
        .chain(&spec.choices)
        .filter_map(|value| {
            let problem = spec.kind.problem(value)?;
            Some(format!(
                "the parameter `{name}` has the value `{value}`, which {problem} for the kind `{}`",
                spec.kind.name()
            ))
        })
        .collect()
}

/// A choice listed twice, and a default that is not one of the choices.
fn choice_problems(name: &str, spec: &ParamSpec) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for choice in &spec.choices {
        if !seen.insert(choice.as_str()) {
            problems.push(format!("the parameter `{name}` lists `{choice}` twice"));
        }
    }
    if !spec.choices.is_empty() && !spec.choices.contains(&spec.default) {
        problems.push(format!(
            "the default `{}` of the parameter `{name}` is not one of its choices",
            spec.default
        ));
    }
    problems
}

/// The placeholders of a template, in order, each as its name and its view, or why one is not a
/// placeholder: a `{{ param.` without its ` }}` on the same line.
pub(crate) fn placeholders(text: &str) -> Result<Vec<(&str, Option<&str>)>, String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find(OPEN) {
        let after = &rest[at + OPEN.len()..];
        let end = after
            .find(CLOSE)
            .filter(|end| !after[..*end].contains('\n'))
            .ok_or_else(|| format!("a `{OPEN}` has no `{CLOSE}` on its line"))?;
        let inner = &after[..end];
        found.push(match inner.split_once('.') {
            Some((name, view)) => (name, Some(view)),
            None => (inner, None),
        });
        rest = &after[end + CLOSE.len()..];
    }
    Ok(found)
}

/// Why the placeholders of `text` do not fit `params`, if they do not: a name that no recipe of
/// the chain declares, or a view that its kind does not have.
pub(crate) fn placeholder_problem(text: &str, params: &[ResolvedParam]) -> Option<String> {
    let found = match placeholders(text) {
        Ok(found) => found,
        Err(problem) => return Some(problem),
    };
    for (name, view) in found {
        let Some(param) = params.iter().find(|param| param.name == name) else {
            return Some(format!(
                "it writes the parameter `{name}`, which no recipe of the chain declares"
            ));
        };
        if let Some(view) = view.filter(|view| !param.spec.kind.views().contains(view)) {
            return Some(format!(
                "it writes `{name}.{view}`, and a parameter of the kind `{}` has {}",
                param.spec.kind.name(),
                match param.spec.kind.views() {
                    [] => "no view".to_owned(),
                    views => format!("the views {}", views.join(", ")),
                }
            ));
        }
    }
    None
}

/// `text` with each placeholder set to the value of its parameter, or why it cannot be.
pub fn render(text: &str, params: &[ResolvedParam]) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    for (name, view) in placeholders(text)? {
        let param = params
            .iter()
            .find(|param| param.name == name)
            .ok_or_else(|| format!("the template writes `{name}`, which no recipe declares"))?;
        let placeholder = match view {
            Some(view) => format!("{OPEN}{name}.{view}{CLOSE}"),
            None => format!("{OPEN}{name}{CLOSE}"),
        };
        let at = rest.find(&placeholder).unwrap_or(rest.len());
        out.push_str(&rest[..at]);
        out.push_str(&param.view(view)?);
        rest = &rest[(at + placeholder.len()).min(rest.len())..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The parameters of botica and the ones that the recipes of `chain` declare, each at its
/// default. Two recipes that declare one name differently are an error.
pub(crate) fn resolve_params(chain: &[&Recipe]) -> Result<Vec<ResolvedParam>, String> {
    let builtin = built_in();
    let mut declared: BTreeMap<&str, ResolvedParam> = builtin
        .iter()
        .map(|param| (param.name.as_str(), param.clone()))
        .collect();
    for recipe in chain {
        for (name, spec) in &recipe.params {
            match declared.get(name.as_str()) {
                Some(first) if first.spec != *spec => {
                    return Err(format!(
                        "`{}` and `{}` declare the parameter `{name}` differently",
                        recipe.id, first.from
                    ));
                }
                Some(_) => {}
                None => {
                    declared.insert(
                        name,
                        ResolvedParam {
                            name: name.clone(),
                            spec: spec.clone(),
                            value: spec.default.clone(),
                            from: recipe.id.clone(),
                        },
                    );
                }
            }
        }
    }
    Ok(declared.into_values().collect())
}

/// Sets the values of `params`: the one of `set` (`--set`), else the one of `config`
/// (`[recipes.params]`), else the default. A name of `set` that `params` does not have is an
/// error; a name of `config` is one only when `declared` says no recipe at all declares it, since
/// the configuration serves every recipe of the project. A value that does not fit is an error.
pub fn set_values(
    params: &mut [ResolvedParam],
    config: &BTreeMap<String, String>,
    set: &BTreeMap<String, String>,
    declared: impl Fn(&str) -> bool,
) -> Result<(), String> {
    let has = |name: &str| params.iter().any(|param| param.name == name);
    if let Some(name) = set.keys().find(|name| !has(name)) {
        return Err(format!(
            "`--set {name}`: no recipe of the ones named declares the parameter `{name}`"
        ));
    }
    if let Some(name) = config.keys().find(|name| !has(name) && !declared(name)) {
        return Err(format!(
            "`[recipes.params]` sets `{name}`, which no recipe declares"
        ));
    }
    for param in params.iter_mut() {
        if let Some(value) = set.get(&param.name).or_else(|| config.get(&param.name)) {
            if let Some(problem) = param.value_problem(value) {
                return Err(problem);
            }
            param.value.clone_from(value);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(name: &str, kind: ParamKind, value: &str) -> ResolvedParam {
        ResolvedParam {
            name: name.to_owned(),
            spec: ParamSpec {
                description: "d".to_owned(),
                default: value.to_owned(),
                kind,
                choices: Vec::new(),
            },
            value: value.to_owned(),
            from: "r".to_owned(),
        }
    }

    #[test]
    fn a_placeholder_takes_its_value_and_the_expressions_of_github_stay() {
        let params = [param("go-preference", ParamKind::Text, "managed")];
        let text = "preference = \"{{ param.go-preference }}\"\nrun: echo ${{ github.ref }}\n";

        let out = render(text, &params).expect("renders");

        assert_eq!(
            out,
            "preference = \"managed\"\nrun: echo ${{ github.ref }}\n"
        );
    }

    #[test]
    fn days_are_written_in_days_minutes_and_seconds() {
        let params = [param("cooldown-days", ParamKind::Days, "7")];
        let text = "a: {{ param.cooldown-days }}\nb: {{ param.cooldown-days.minutes }}\n\
                    c: {{ param.cooldown-days.seconds }}\n";

        let out = render(text, &params).expect("renders");

        assert_eq!(out, "a: 7\nb: 10080\nc: 604800\n");
    }

    #[test]
    fn an_empty_version_writes_no_at_and_cannot_fill_a_bare_placeholder() {
        let empty = [param("pyrefly-version", ParamKind::Version, "")];
        let pinned = [param("pyrefly-version", ParamKind::Version, "0.40.0")];
        let text = "uvx pyrefly{{ param.pyrefly-version.at }} check";

        assert_eq!(render(text, &empty).expect("renders"), "uvx pyrefly check");
        assert_eq!(
            render(text, &pinned).expect("renders"),
            "uvx pyrefly@0.40.0 check"
        );
        assert!(render("rev = \"v{{ param.pyrefly-version }}\"", &empty).is_err());
    }

    #[test]
    fn a_name_the_chain_lacks_a_view_its_kind_lacks_and_an_open_placeholder_are_problems() {
        let params = [param("go-preference", ParamKind::Text, "system")];

        assert!(placeholder_problem("{{ param.days }}", &params).is_some());
        assert!(placeholder_problem("{{ param.go-preference.at }}", &params).is_some());
        assert!(placeholder_problem("{{ param.go-preference\n }}", &params).is_some());
        assert!(placeholder_problem("{{ param.go-preference }}", &params).is_none());
    }

    #[test]
    fn set_wins_over_the_configuration_which_wins_over_the_default() {
        let mut params = [
            param("a", ParamKind::Days, "7"),
            param("b", ParamKind::Days, "7"),
            param("c", ParamKind::Days, "7"),
        ];
        let config = BTreeMap::from([
            ("a".to_owned(), "14".to_owned()),
            ("b".to_owned(), "14".to_owned()),
        ]);
        let set = BTreeMap::from([("a".to_owned(), "21".to_owned())]);

        set_values(&mut params, &config, &set, |_| false).expect("sets");

        let values: Vec<&str> = params.iter().map(|param| param.value.as_str()).collect();
        assert_eq!(values, ["21", "14", "7"]);
    }

    #[test]
    fn an_unknown_name_and_a_value_out_of_its_kind_are_errors() {
        let mut params = [param("a", ParamKind::Days, "7")];
        let empty = BTreeMap::new();
        let other = BTreeMap::from([("z".to_owned(), "1".to_owned())]);
        let bad = BTreeMap::from([("a".to_owned(), "a week".to_owned())]);

        assert!(set_values(&mut params, &empty, &other, |_| true).is_err());
        assert!(set_values(&mut params, &other, &empty, |_| false).is_err());
        assert!(set_values(&mut params, &other, &empty, |_| true).is_ok());
        assert!(set_values(&mut params, &empty, &bad, |_| true).is_err());
    }

    #[test]
    fn a_default_out_of_its_choices_or_its_kind_is_a_problem() {
        let params = BTreeMap::from([(
            "go-preference".to_owned(),
            ParamSpec {
                description: "The Go of prek".to_owned(),
                default: "mine".to_owned(),
                kind: ParamKind::Text,
                choices: vec!["system".to_owned(), "a\nb".to_owned()],
            },
        )]);
        let days = BTreeMap::from([(
            "cooldown-days".to_owned(),
            ParamSpec {
                description: "Days".to_owned(),
                default: "a week".to_owned(),
                kind: ParamKind::Days,
                choices: Vec::new(),
            },
        )]);

        assert_eq!(param_problems(&params).len(), 2);
        assert_eq!(param_problems(&days).len(), 1);
    }

    #[test]
    fn every_chain_has_botica_version_at_the_release_of_the_binary_and_no_recipe_declares_it() {
        let params = resolve_params(&[]).expect("resolves");
        let declared = BTreeMap::from([(
            BOTICA_VERSION.to_owned(),
            ParamSpec {
                description: "d".to_owned(),
                default: "1.0.0".to_owned(),
                kind: ParamKind::Version,
                choices: Vec::new(),
            },
        )]);

        assert_eq!(params.len(), 1);
        assert_eq!(params[0].value, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            render("uvx botica@{{ param.botica-version }} lint", &params).expect("renders"),
            format!("uvx botica@{} lint", env!("CARGO_PKG_VERSION"))
        );
        assert_eq!(param_problems(&declared).len(), 1);
    }
}
