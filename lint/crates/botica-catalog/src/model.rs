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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    File,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Needs {
    Path,
    Bytes,
    Syntax,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunOn {
    Cli,
    Save,
    Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Native,
    AstGrep,
    Extension,
    Judgment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Runtime {
    Python,
    Node,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OptionType {
    Int,
    Bool,
    String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptionSpec {
    #[serde(rename = "type")]
    pub kind: OptionType,
    pub default: toml::Value,
    pub min: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Implementation {
    pub kind: Kind,
    pub runtime: Option<Runtime>,
    pub languages: Vec<String>,
    pub status: Option<Status>,
    pub file: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Family {
    pub name: String,
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Rule {
    pub id: String,
    pub code: String,
    pub standard: Option<String>,
    pub summary: String,
    pub status: Status,
    pub since: String,
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
    pub scope: Scope,
    pub needs: Needs,
    pub run_on: Vec<RunOn>,
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
