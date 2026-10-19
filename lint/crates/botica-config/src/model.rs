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

/// Settings of `botica refs`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ReferencesConfig {
    /// Ids of the built-in topics that a topic of `references/` replaces. A topic of the project
    /// with the id of a built-in one is an error unless the id is listed here.
    #[serde(default)]
    pub replace: Vec<String>,
}

/// Settings of `botica recipe`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RecipesConfig {
    /// Ids of the built-in recipes that a recipe of `recipes/` replaces. A recipe of the project
    /// with the id of a built-in one is an error unless the id is listed here.
    #[serde(default)]
    pub replace: Vec<String>,
}

/// What `botica lint` does when the process of an extension rule does not answer in time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OnTimeout {
    /// The run fails with exit code 2.
    #[default]
    Fail,
    /// A warning on stderr; the run goes on without the findings of that rule.
    Warn,
}

/// Settings of the extension rules.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ExtensionsConfig {
    /// The Python of the scripts of the runtime `python`: `project` (the version of
    /// `.python-version`, of `.venv` or of `requires-python`, else 3.13), `system` (a Python of
    /// the system) or a version for uv, as `3.12`. A free-threaded Python is refused.
    pub python: Option<String>,
    /// Seconds that a process has to answer each request. `BOTICA_EXTENSION_TIMEOUT` wins over it.
    pub timeout: Option<u64>,
    #[serde(default)]
    pub on_timeout: OnTimeout,
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
    /// Globs of paths to lint although `.gitignore` or the list of directories of tools (`.venv`,
    /// `.mypy_cache`…) would leave them out. `extend-exclude` wins over it.
    #[serde(default)]
    pub extend_include: Vec<String>,
    /// Where the code of the project lives, as directories from its root grouped by ecosystem
    /// (`python`, `js`, `rust`, `go`, `jvm`), for the rules about layout. When absent they are
    /// detected; an empty table says there are none.
    #[serde(default)]
    pub code_dirs: Option<BTreeMap<String, Vec<String>>>,
    /// Directories with the project's own rules.
    #[serde(default = "default_rule_dirs")]
    pub rule_dirs: Vec<String>,
    /// Globs of rule IDs that suppressions may name without a warning even when they are unknown.
    #[serde(default)]
    pub external: Vec<String>,
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
    #[serde(default)]
    pub references: ReferencesConfig,
    #[serde(default)]
    pub recipes: RecipesConfig,
    #[serde(default)]
    pub extensions: ExtensionsConfig,
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
            extend_include: Vec::new(),
            code_dirs: None,
            rule_dirs: default_rule_dirs(),
            external: Vec::new(),
            catalogs: BTreeMap::new(),
            replace: BTreeMap::new(),
            rules: BTreeMap::new(),
            overrides: Vec::new(),
            references: ReferencesConfig::default(),
            recipes: RecipesConfig::default(),
            extensions: ExtensionsConfig::default(),
        }
    }
}
