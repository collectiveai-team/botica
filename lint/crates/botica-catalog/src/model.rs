use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Preview,
    Stable,
    Deprecated,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    File,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Needs {
    Path,
    Bytes,
    Syntax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunOn {
    Cli,
    Save,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Native,
    AstGrep,
    Extension,
    Judgment,
}

/// When a judge evaluates a judgment rule: on each edit of a file, at the end of a turn of an
/// agent, or on a pull request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum When {
    Edit,
    Turn,
    PullRequest,
}

/// The question a judge answers about a rule, in the form of `@coldtea/abide-schema` 0.0.5: yes or
/// no, one of several options, or a level of a score.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Question {
    Boolean {
        instructions: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        criteria: Option<BooleanCriteria>,
    },
    Choice {
        instructions: String,
        criteria: Choices,
        violating: Vec<String>,
    },
    Score {
        instructions: String,
        /// The levels, from compliant (the first) to the worst.
        criteria: Vec<String>,
        /// The first level that is a violation.
        #[serde(rename = "violatingFrom")]
        violating_from: usize,
    },
}

/// What a yes and a no to a boolean question mean.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanCriteria {
    #[serde(rename = "true", default, skip_serializing_if = "Option::is_none")]
    pub yes: Option<String>,
    #[serde(rename = "false", default, skip_serializing_if = "Option::is_none")]
    pub no: Option<String>,
}

/// The options of a choice question, each with its text, in the order of `rule.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choices(pub Vec<(String, String)>);

impl<'de> Deserialize<'de> for Choices {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Choices;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a table of options, each with its text")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Choices, A::Error> {
                let mut options = Vec::new();
                while let Some(option) = map.next_entry()? {
                    options.push(option);
                }
                Ok(Choices(options))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

impl Serialize for Choices {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(name, text)| (name, text)))
    }
}

impl Question {
    pub fn instructions(&self) -> &str {
        match self {
            Self::Boolean { instructions, .. }
            | Self::Choice { instructions, .. }
            | Self::Score { instructions, .. } => instructions,
        }
    }

    /// The texts of its criteria, in their order.
    pub fn criteria(&self) -> Vec<&str> {
        match self {
            Self::Boolean { criteria, .. } => criteria
                .iter()
                .flat_map(|criteria| [&criteria.yes, &criteria.no])
                .flatten()
                .map(String::as_str)
                .collect(),
            Self::Choice { criteria, .. } => {
                criteria.0.iter().map(|(_, text)| text.as_str()).collect()
            }
            Self::Score { criteria, .. } => criteria.iter().map(String::as_str).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Runtime {
    Python,
    Node,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum OptionType {
    Int,
    Bool,
    String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OptionSpec {
    #[serde(rename = "type")]
    pub kind: OptionType,
    pub default: toml::Value,
    pub min: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Implementation {
    pub kind: Kind,
    pub runtime: Option<Runtime>,
    pub languages: Vec<String>,
    pub status: Option<Status>,
    pub file: Option<String>,
    /// The fields of a judgment implementation.
    pub when: Option<When>,
    pub question: Option<Question>,
    #[serde(default)]
    pub covered_by: Vec<String>,
    pub why_not_deterministic: Option<String>,
    /// The text of `file`, read by the catalog when it loads (never written in `rule.toml`).
    #[serde(skip)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FamilyFile {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Family {
    pub name: String,
    pub code: String,
    pub description: String,
}

/// One line of `history.toml`: from `version` on, `rule` has `status`. The file is written by the
/// release, never by hand.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub version: String,
    pub rule: String,
    pub status: Status,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HistoryFile {
    #[serde(default)]
    pub event: Vec<Event>,
}

/// When a rule was released: the version that added it and the one that gave it its current
/// status. A `None` is a change that no release has recorded yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lifecycle {
    pub added: Option<String>,
    pub since: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Rule {
    pub id: String,
    pub code: String,
    pub standard: Option<String>,
    pub summary: String,
    pub status: Status,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub replaces: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub ignores: Vec<String>,
    pub prefilter: Option<String>,
    /// `scope`, `needs` and `run-on` are required in a deterministic rule and absent in a
    /// judgment rule.
    pub scope: Option<Scope>,
    pub needs: Option<Needs>,
    pub run_on: Option<Vec<RunOn>>,
    pub default_enabled: bool,
    pub default_severity: Severity,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub options: BTreeMap<String, OptionSpec>,
    #[serde(default)]
    pub implementations: Vec<Implementation>,
    #[serde(skip)]
    pub family: Option<String>,
    #[serde(skip)]
    pub readme: String,
}

impl Rule {
    /// Whether a judge evaluates the rule: its implementation is of kind `judgment`.
    pub fn is_judgment(&self) -> bool {
        self.implementations
            .iter()
            .any(|implementation| implementation.kind == Kind::Judgment)
    }

    pub fn status_of(&self, implementation: &Implementation) -> Status {
        implementation.status.unwrap_or(self.status)
    }

    pub fn languages(&self) -> Vec<&str> {
        let mut all: Vec<&str> = self
            .implementations
            .iter()
            .flat_map(|implementation| implementation.languages.iter().map(String::as_str))
            .collect();
        all.sort_unstable();
        all.dedup();
        all
    }
}
