use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RuleSeverity {
    Error,
    Warning,
    Info,
    Off,
}

/// A catalog of rules that lives on disk.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogSource {
    /// Directory of the catalog, relative to the directory of this file.
    pub path: String,
}

/// Settings of one rule. Any other key is an option declared by that rule.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct RuleConfig {
    /// `off` is the same as `enabled = false`.
    pub severity: Option<RuleSeverity>,
    pub enabled: Option<bool>,
    /// Replaces the default `ignores` of the rule (`[]` clears them).
    pub ignores: Option<Vec<String>>,
    /// Added to the `ignores` that apply.
    #[serde(default)]
    pub extend_ignores: Vec<String>,
    #[serde(flatten)]
    #[schemars(with = "BTreeMap<String, serde_json::Value>")]
    pub options: BTreeMap<String, toml::Value>,
}

/// Changes for the files that match `files`, applied in order.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Override {
    pub files: Vec<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub severity: BTreeMap<String, RuleSeverity>,
}

/// Settings of `botica lint`: `botica.toml`, or the `[tool.botica]` table of `pyproject.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    /// Rules to run: `default`, `all`, a family (`BTSTRUC`), a code prefix (`BTSTRUC0`), a code, a slug or `tag:<tag>`.
    #[serde(default = "default_select")]
    pub select: Vec<String>,
    /// Added to `select`.
    #[serde(default)]
    pub extend_select: Vec<String>,
    /// Rules to switch off. Same syntax as `select`.
    #[serde(default)]
    pub ignore: Vec<String>,
    /// Also run the rules whose status is `preview`.
    #[serde(default)]
    pub preview: bool,
    /// Restrict the run to these languages (empty means every language).
    #[serde(default)]
    pub languages: Vec<String>,
    /// Globs of paths to skip, in addition to what `.gitignore` already skips.
    #[serde(default)]
    pub extend_exclude: Vec<String>,
    /// The packages of the project, as directories from its root, for the rules about layout. When
    /// absent they are detected; `[]` says there are none.
    #[serde(default)]
    pub packages: Option<Vec<String>>,
    /// Directories with the project's own rules.
    #[serde(default = "default_rule_dirs")]
    pub rule_dirs: Vec<String>,
    /// Globs of rule IDs that suppressions may name without a warning even when they are unknown.
    #[serde(default)]
    pub external: Vec<String>,
    /// While migrating from ast-grep: honour `# ast-grep-ignore: <rule>` on the line of a finding and
    /// on the line before it.
    #[serde(default)]
    pub compat_ast_grep_ignore: bool,
    #[serde(default)]
    pub catalogs: BTreeMap<String, CatalogSource>,
    /// Rule to replace -> rule that replaces it.
    #[serde(default)]
    pub replace: BTreeMap<String, String>,
    /// Settings per rule, keyed by slug, code or alias.
    #[serde(default)]
    pub rules: BTreeMap<String, RuleConfig>,
    #[serde(default)]
    pub overrides: Vec<Override>,
}

fn default_select() -> Vec<String> {
    vec!["default".to_owned()]
}

fn default_rule_dirs() -> Vec<String> {
    vec![".botica/rules".to_owned()]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            select: default_select(),
            extend_select: Vec::new(),
            ignore: Vec::new(),
            preview: false,
            languages: Vec::new(),
            extend_exclude: Vec::new(),
            packages: None,
            rule_dirs: default_rule_dirs(),
            external: Vec::new(),
            compat_ast_grep_ignore: false,
            catalogs: BTreeMap::new(),
            replace: BTreeMap::new(),
            rules: BTreeMap::new(),
            overrides: Vec::new(),
        }
    }
}
