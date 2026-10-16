//! `external-config`: botica checks that the tool of an active external rule is set up, by
//! finding the fragment of configuration the rule declares in the file of the project it names.

use botica_catalog::{Kind, Rule};

use crate::diagnostic::{Diagnostic, Severity};

pub const EXTERNAL_CONFIG: &str = "external-config";

/// A fragment of configuration that a rule needs in a file of the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement<'a> {
    pub rule: &'a str,
    pub standard: Option<&'a str>,
    pub tool: &'a str,
    /// The file, from the root of the project.
    pub target: &'a str,
    pub fragment: &'a str,
}

impl<'a> Requirement<'a> {
    /// The fragments that the external implementations of `rule` declare.
    pub fn of(rule: &'a Rule) -> Vec<Self> {
        rule.implementations
            .iter()
            .filter(|implementation| implementation.kind == Kind::External)
            .filter_map(|implementation| {
                Some(Self {
                    rule: &rule.id,
                    standard: rule.standard.as_deref(),
                    tool: implementation.tool.as_deref()?,
                    target: implementation.target()?,
                    fragment: implementation.source.as_deref()?,
                })
            })
            .collect()
    }

    fn label(&self) -> String {
        match self.standard {
            Some(standard) => format!("`{}` ({standard})", self.rule),
            None => format!("`{}`", self.rule),
        }
    }
}

/// One finding for each requirement whose file is missing, is not TOML, or lacks the fragment.
/// `read` gives the text of a file from the root of the project, `None` when it does not exist.
pub fn check(
    requirements: &[Requirement<'_>],
    read: impl Fn(&str) -> Option<String>,
    severity: Severity,
) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    for requirement in requirements {
        let (target, tool, label) = (requirement.target, requirement.tool, requirement.label());
        let explain = format!("`botica explain {}`", requirement.rule);
        let message = match read(target).map(|text| text.parse::<toml::Table>()) {
            None => format!(
                "{target} does not exist, and {tool} needs it for {label}: {explain} shows what \
                 goes in it"
            ),
            Some(Err(error)) => format!(
                "{target} is not TOML, so botica cannot find the configuration of {tool} that \
                 {label} needs: {}",
                error.message()
            ),
            Some(Ok(table)) => match requirement.fragment.parse::<toml::Table>() {
                Ok(fragment) if holds_table(&table, &fragment) => continue,
                _ => format!(
                    "{target} lacks the configuration of {tool} that {label} needs: {explain} \
                     shows it"
                ),
            },
        };
        found.push(Diagnostic::file_level(
            target,
            EXTERNAL_CONFIG,
            severity,
            message,
        ));
    }
    found
}

/// Whether `value` holds `fragment`: a table has each key of the fragment with a value that holds
/// the value of the fragment, an array has, for each item of the fragment, an item that holds it,
/// and any other value is equal.
fn holds(value: &toml::Value, fragment: &toml::Value) -> bool {
    match (value, fragment) {
        (toml::Value::Table(table), toml::Value::Table(wanted)) => holds_table(table, wanted),
        (toml::Value::Array(items), toml::Value::Array(wanted)) => wanted
            .iter()
            .all(|needed| items.iter().any(|item| holds(item, needed))),
        _ => value == fragment,
    }
}

fn holds_table(table: &toml::Table, fragment: &toml::Table) -> bool {
    fragment
        .iter()
        .all(|(key, wanted)| table.get(key).is_some_and(|value| holds(value, wanted)))
}
