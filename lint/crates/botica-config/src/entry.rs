use std::collections::BTreeMap;
use std::fmt;

use botica_catalog::{Catalog, Implementation, Rule, Severity, Status};
use serde::Serialize;

use crate::issue::ConfigError;
use crate::matcher::PathMatcher;
use crate::model::{Config, RuleConfig, RuleSeverity};
use crate::resolve::Selectors;

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
    /// Left out by `--scope`; holds the scope that was asked for.
    OutOfScope(String),
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
            Self::OutOfScope(scope) => write!(f, "outside `--scope {scope}`"),
        }
    }
}

/// Where an ignore glob of a rule is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IgnoreOrigin {
    /// The `rule.toml` of the rule.
    Rule,
    /// `ignores` or `extend-ignores` of `rules.<id>` in the configuration.
    Config,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnorePattern {
    pub glob: String,
    pub origin: IgnoreOrigin,
}

#[derive(Debug)]
pub struct Entry {
    pub rule: Rule,
    pub active: bool,
    pub reason: Reason,
    pub severity: Severity,
    /// The severity `rules.<id>` names, if it names one (`severity` falls back to the catalog's).
    pub configured_severity: Option<Severity>,
    pub options: BTreeMap<String, toml::Value>,
    /// The ignores that apply, in order, each with where it is written.
    pub ignore_patterns: Vec<IgnorePattern>,
    pub(crate) ignores: PathMatcher,
    pub(crate) files: PathMatcher,
}

pub(crate) fn first_match(catalog: &Catalog, selectors: &Selectors, rule: &Rule) -> Option<String> {
    selectors
        .iter()
        .find(|(_, selector)| selector.matches(catalog, rule))
        .map(|(text, _)| text.clone())
}

pub(crate) fn runs(rule: &Rule, implementation: &Implementation, preview: bool) -> bool {
    preview || rule.status_of(implementation) != Status::Preview
}

pub(crate) fn severity_of(severity: RuleSeverity) -> Option<Severity> {
    match severity {
        RuleSeverity::Error => Some(Severity::Error),
        RuleSeverity::Warning => Some(Severity::Warning),
        RuleSeverity::Info => Some(Severity::Info),
        RuleSeverity::Off => None,
    }
}

pub(crate) fn effective_options(
    rule: &Rule,
    config: Option<&RuleConfig>,
) -> BTreeMap<String, toml::Value> {
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

pub(crate) fn matcher(
    patterns: &[String],
    rule: &Rule,
    what: &str,
) -> Result<PathMatcher, ConfigError> {
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

pub(crate) fn entry(
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
    let (own_ignores, own_origin) = match rc.and_then(|c| c.ignores.as_ref()) {
        Some(replacement) => (replacement.clone(), IgnoreOrigin::Config),
        None => (rule.ignores.clone(), IgnoreOrigin::Rule),
    };
    let ignore_patterns: Vec<IgnorePattern> = own_ignores
        .into_iter()
        .map(|glob| IgnorePattern {
            glob,
            origin: own_origin,
        })
        .chain(
            rc.map(|c| c.extend_ignores.clone())
                .unwrap_or_default()
                .into_iter()
                .map(|glob| IgnorePattern {
                    glob,
                    origin: IgnoreOrigin::Config,
                }),
        )
        .collect();
    let ignores: Vec<String> = ignore_patterns
        .iter()
        .map(|pattern| pattern.glob.clone())
        .collect();
    Ok(Entry {
        rule: rule.clone(),
        active,
        reason,
        severity: configured.unwrap_or(rule.default_severity),
        configured_severity: configured,
        options: effective_options(rule, rc),
        ignore_patterns,
        ignores: matcher(&ignores, rule, "ignores")?,
        files: matcher(&rule.files, rule, "files")?,
    })
}
