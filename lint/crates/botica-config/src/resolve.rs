use std::collections::BTreeMap;
use std::fmt;

use botica_catalog::{Catalog, Implementation, Rule, Severity, Status};

use crate::issue::ConfigError;
use crate::matcher::PathMatcher;
use crate::model::{Config, RuleConfig, RuleSeverity};
use crate::selector::Selector;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    SelectedBy(String),
    ReplacementOf(String),
    NotSelected,
    IgnoredBy(String),
    Disabled,
    SeverityOff,
    OnlyPreview,
    Removed,
    ReplacedBy(String),
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SelectedBy(selector) => write!(f, "selected by `{selector}`"),
            Self::ReplacementOf(old) => write!(f, "replaces `{old}`"),
            Self::NotSelected => write!(f, "not selected"),
            Self::IgnoredBy(selector) => write!(f, "ignored by `{selector}`"),
            Self::Disabled => write!(f, "`enabled = false` in the configuration"),
            Self::SeverityOff => write!(f, "`severity = \"off\"` in the configuration"),
            Self::OnlyPreview => write!(
                f,
                "only preview implementations (set `preview = true` to run them)"
            ),
            Self::Removed => write!(f, "removed from the catalog"),
            Self::ReplacedBy(new) => write!(f, "replaced by `{new}`"),
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    pub rule: Rule,
    pub active: bool,
    pub reason: Reason,
    pub severity: Severity,
    pub options: BTreeMap<String, toml::Value>,
    ignores: PathMatcher,
    files: PathMatcher,
}

#[derive(Debug)]
pub struct FileRule<'r> {
    pub rule: &'r Rule,
    pub severity: Severity,
    pub implementations: Vec<&'r Implementation>,
    pub options: &'r BTreeMap<String, toml::Value>,
}

#[derive(Debug)]
struct CompiledOverride {
    files: PathMatcher,
    ignore: Vec<Selector>,
    severity: Vec<(String, RuleSeverity)>,
}

#[derive(Debug)]
pub struct Resolution {
    catalog: Catalog,
    entries: Vec<Entry>,
    languages: Vec<String>,
    preview: bool,
    overrides: Vec<CompiledOverride>,
    external: PathMatcher,
    compat_ast_grep_ignore: bool,
}

type Selectors = Vec<(String, Selector)>;

fn selectors_of(values: &[String]) -> Selectors {
    values
        .iter()
        .filter_map(|text| {
            Selector::parse(text)
                .ok()
                .map(|selector| (text.clone(), selector))
        })
        .collect()
}

fn first_match(catalog: &Catalog, selectors: &Selectors, rule: &Rule) -> Option<String> {
    selectors
        .iter()
        .find(|(_, selector)| selector.matches(catalog, rule))
        .map(|(text, _)| text.clone())
}

fn runs(rule: &Rule, implementation: &Implementation, preview: bool) -> bool {
    preview || rule.status_of(implementation) != Status::Preview
}

fn severity_of(severity: RuleSeverity) -> Option<Severity> {
    match severity {
        RuleSeverity::Error => Some(Severity::Error),
        RuleSeverity::Warning => Some(Severity::Warning),
        RuleSeverity::Info => Some(Severity::Info),
        RuleSeverity::Off => None,
    }
}

fn effective_options(rule: &Rule, config: Option<&RuleConfig>) -> BTreeMap<String, toml::Value> {
    let mut options: BTreeMap<String, toml::Value> = rule
        .options
        .iter()
        .map(|(name, spec)| (name.clone(), spec.default.clone()))
        .collect();
    if let Some(config) = config {
        options.extend(config.options.clone());
    }
    options
}

fn matcher(patterns: &[String], rule: &Rule, what: &str) -> Result<PathMatcher, ConfigError> {
    PathMatcher::new(patterns).map_err(|errors| {
        let detail = errors
            .iter()
            .map(|(_, message)| message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        ConfigError::single(
            format!("rule {}", rule.id),
            format!("invalid `{what}` pattern: {detail}"),
        )
    })
}

impl Resolution {
    pub fn new(config: &Config, catalog: &Catalog) -> Result<Self, ConfigError> {
        config.validate(catalog)?;
        let select: Selectors = selectors_of(&config.select)
            .into_iter()
            .chain(selectors_of(&config.extend_select))
            .collect();
        let ignore = selectors_of(&config.ignore);
        let per_rule: BTreeMap<&str, &RuleConfig> = config
            .rules
            .iter()
            .filter_map(|(key, rc)| catalog.resolve(key).map(|rule| (rule.id.as_str(), rc)))
            .collect();

        let mut entries = Vec::with_capacity(catalog.rules().len());
        for rule in catalog.rules() {
            let rc = per_rule.get(rule.id.as_str()).copied();
            entries.push(entry(catalog, config, rule, rc, &select, &ignore)?);
        }
        let mut resolution = Self {
            catalog: catalog.clone(),
            entries,
            languages: config.languages.clone(),
            preview: config.preview,
            overrides: compile_overrides(config, catalog)?,
            external: PathMatcher::new(&config.external).map_err(|errors| {
                let detail = errors
                    .iter()
                    .map(|(_, message)| message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                ConfigError::single("external", detail)
            })?,
            compat_ast_grep_ignore: config.compat_ast_grep_ignore,
        };
        resolution.apply_replacements(config)?;
        Ok(resolution)
    }

    fn apply_replacements(&mut self, config: &Config) -> Result<(), ConfigError> {
        for (old_key, new_key) in &config.replace {
            let (Some(old), Some(new)) =
                (self.catalog.resolve(old_key), self.catalog.resolve(new_key))
            else {
                continue;
            };
            let declared = new
                .replaces
                .as_deref()
                .and_then(|id| self.catalog.resolve(id));
            if declared.map(|rule| &rule.id) != Some(&old.id) {
                return Err(ConfigError::single(
                    format!("replace.{old_key}"),
                    format!("rule `{}` does not declare that it replaces `{}` (`replaces` in its rule.toml)", new.id, old.id),
                ));
            }
            let old_is_active = self.entries.iter().any(|e| e.rule.id == old.id && e.active);
            if !old_is_active {
                continue;
            }
            for entry in &mut self.entries {
                if entry.rule.id == old.id {
                    entry.active = false;
                    entry.reason = Reason::ReplacedBy(new.id.clone());
                } else if entry.rule.id == new.id && entry.reason == Reason::NotSelected {
                    entry.active = true;
                    entry.reason = Reason::ReplacementOf(old.id.clone());
                }
            }
        }
        Ok(())
    }

    /// Whether a suppression may name `id` without a warning although no rule has it.
    pub fn is_external(&self, id: &str) -> bool {
        self.external.is_match(id)
    }

    pub fn compat_ast_grep_ignore(&self) -> bool {
        self.compat_ast_grep_ignore
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn entry(&self, key: &str) -> Option<&Entry> {
        let rule = self.catalog.resolve(key)?;
        self.entries.iter().find(|entry| entry.rule.id == rule.id)
    }

    pub fn rules_for<'r>(&'r self, path: &str, language: Option<&str>) -> Vec<FileRule<'r>> {
        if !self.languages.is_empty()
            && !language.is_some_and(|l| self.languages.iter().any(|x| x == l))
        {
            return Vec::new();
        }
        let mut found: Vec<FileRule<'r>> = self
            .entries
            .iter()
            .filter(|entry| entry.active)
            .filter(|entry| entry.files.is_empty() || entry.files.is_match(path))
            .filter(|entry| !entry.ignores.is_match(path))
            .filter_map(|entry| {
                let implementations: Vec<&Implementation> = entry
                    .rule
                    .implementations
                    .iter()
                    .filter(|i| runs(&entry.rule, i, self.preview))
                    .filter(|i| {
                        language.map_or_else(
                            || i.languages.iter().any(|l| l == "*"),
                            |l| i.languages.iter().any(|x| x == "*" || x == l),
                        )
                    })
                    .collect();
                (!implementations.is_empty()).then_some(FileRule {
                    rule: &entry.rule,
                    severity: entry.severity,
                    implementations,
                    options: &entry.options,
                })
            })
            .collect();
        for over in self.overrides.iter().filter(|o| o.files.is_match(path)) {
            found.retain(|file_rule| {
                !over
                    .ignore
                    .iter()
                    .any(|s| s.matches(&self.catalog, file_rule.rule))
            });
            for (id, severity) in &over.severity {
                match severity_of(*severity) {
                    Some(level) => {
                        if let Some(file_rule) = found.iter_mut().find(|r| r.rule.id == *id) {
                            file_rule.severity = level;
                        }
                    }
                    None => found.retain(|r| r.rule.id != *id),
                }
            }
        }
        found
    }
}

fn entry(
    catalog: &Catalog,
    config: &Config,
    rule: &Rule,
    rc: Option<&RuleConfig>,
    select: &Selectors,
    ignore: &Selectors,
) -> Result<Entry, ConfigError> {
    let selected = first_match(catalog, select, rule);
    let any_runs = rule
        .implementations
        .iter()
        .any(|i| runs(rule, i, config.preview));
    let reason = if rule.status == Status::Removed {
        Reason::Removed
    } else if let Some(by) = first_match(catalog, ignore, rule) {
        Reason::IgnoredBy(by)
    } else if rc.is_some_and(|c| c.enabled == Some(false)) {
        Reason::Disabled
    } else if rc.is_some_and(|c| c.severity == Some(RuleSeverity::Off)) {
        Reason::SeverityOff
    } else {
        match selected {
            None => Reason::NotSelected,
            Some(_) if !any_runs => Reason::OnlyPreview,
            Some(by) => Reason::SelectedBy(by),
        }
    };
    let active = matches!(reason, Reason::SelectedBy(_));
    let configured = rc.and_then(|c| c.severity).and_then(severity_of);
    let own_ignores = match rc.and_then(|c| c.ignores.as_ref()) {
        Some(replacement) => replacement.clone(),
        None => rule.ignores.clone(),
    };
    let ignores: Vec<String> = own_ignores
        .into_iter()
        .chain(rc.map(|c| c.extend_ignores.clone()).unwrap_or_default())
        .collect();
    Ok(Entry {
        rule: rule.clone(),
        active,
        reason,
        severity: configured.unwrap_or(rule.default_severity),
        options: effective_options(rule, rc),
        ignores: matcher(&ignores, rule, "ignores")?,
        files: matcher(&rule.files, rule, "files")?,
    })
}

fn compile_overrides(
    config: &Config,
    catalog: &Catalog,
) -> Result<Vec<CompiledOverride>, ConfigError> {
    config
        .overrides
        .iter()
        .enumerate()
        .map(|(index, over)| {
            let files = PathMatcher::new(&over.files).map_err(|errors| {
                let detail = errors
                    .iter()
                    .map(|(_, m)| m.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                ConfigError::single(format!("overrides[{index}].files"), detail)
            })?;
            Ok(CompiledOverride {
                files,
                ignore: selectors_of(&over.ignore)
                    .into_iter()
                    .map(|(_, s)| s)
                    .collect(),
                severity: over
                    .severity
                    .iter()
                    .filter_map(|(key, sev)| {
                        catalog.resolve(key).map(|rule| (rule.id.clone(), *sev))
                    })
                    .collect(),
            })
        })
        .collect()
}
