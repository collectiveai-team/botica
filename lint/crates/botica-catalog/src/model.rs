use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::question::Question;

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
    External,
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
    /// The program and arguments of an extension with the runtime `command`, run from the root of
    /// the project.
    pub command: Option<Vec<String>>,
    /// The fields of a judgment implementation.
    pub when: Option<When>,
    pub question: Option<Question>,
    #[serde(default)]
    pub covered_by: Vec<String>,
    pub why_not_deterministic: Option<String>,
    /// The fields of an external implementation: the tool that enforces the rule, the address of
    /// its documentation, its codes, and the fragment of configuration it needs,
    /// `config/<file>.toml`.
    pub tool: Option<String>,
    pub docs: Option<String>,
    #[serde(default)]
    pub codes: Vec<String>,
    pub config: Option<String>,
    /// The text of `file` or of `config`, read by the catalog when it loads (never written in
    /// `rule.toml`).
    #[serde(skip)]
    pub source: Option<String>,
}

/// The folder of a rule that holds the fragments of configuration of its external
/// implementations.
pub const CONFIG_DIR: &str = "config/";

impl Implementation {
    /// The file of the project that the fragment of `config` goes into, from the root of the
    /// project: `config/prek.toml` goes into `prek.toml`.
    pub fn target(&self) -> Option<&str> {
        self.config.as_deref()?.strip_prefix(CONFIG_DIR)
    }
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
    /// What the code of each rule of the family starts with before the family code: `BT` for the
    /// built-in catalog, the code of the catalog for a catalog of a project.
    #[serde(skip)]
    pub header: String,
    /// The catalog of a project that holds the family; `None` for the built-in catalog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catalog: Option<String>,
}

impl Family {
    /// The letters that every code of the family starts with: `BTSTRUC`, `ACMESTRUC`.
    pub fn prefix(&self) -> String {
        format!("{}{}", self.header, self.code)
    }
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
    /// The catalog of a project that holds the rule; `None` for the built-in catalog.
    #[serde(skip)]
    pub catalog: Option<String>,
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

    /// Whether another tool enforces the whole rule: every implementation is of kind `external`,
    /// so `botica lint` runs none of it.
    pub fn is_external_only(&self) -> bool {
        !self.implementations.is_empty()
            && self
                .implementations
                .iter()
                .all(|implementation| implementation.kind == Kind::External)
    }

    /// Whether a program of the project runs the rule, in one language at least.
    pub fn has_extension(&self) -> bool {
        self.implementations
            .iter()
            .any(|implementation| implementation.kind == Kind::Extension)
    }

    /// Whether another tool enforces the rule, in one language at least.
    pub fn has_external(&self) -> bool {
        self.implementations
            .iter()
            .any(|implementation| implementation.kind == Kind::External)
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
