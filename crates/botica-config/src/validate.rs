use std::collections::{BTreeMap, BTreeSet};

use botica_catalog::{Catalog, Ecosystem, OptionSpec, OptionType, Rule};

use crate::issue::{issue, ConfigError, ConfigIssue};
use crate::matcher::PathMatcher;
use crate::model::{Config, Override, RuleConfig};
use crate::selector::{read_rule, Selector, Use};

impl Config {
    /// Checks the configuration against `catalog`, and returns the warnings that reading it
    /// printed: a retired name, or a removed rule where it changes nothing (ADR 0016). `turn_on`
    /// is how `select` and `extend-select` use a rule: [`Use::TurnsOn`], or
    /// [`Use::TurnsOnDescribed`] for a command that describes the configuration.
    pub fn validate(&self, catalog: &Catalog, turn_on: Use) -> Result<Vec<String>, ConfigError> {
        let mut reader = Reader {
            catalog,
            found: Vec::new(),
            warnings: BTreeSet::new(),
        };
        reader.selectors("select", &self.select, turn_on);
        reader.selectors("extend-select", &self.extend_select, turn_on);
        reader.selectors("ignore", &self.ignore, Use::Other);
        let found = &mut reader.found;
        globs(found, "extend-exclude", &self.extend_exclude);
        globs(found, "extend-include", &self.extend_include);
        globs(found, "external", &self.external);
        code_dirs(found, self.code_dirs.as_ref());
        for (old, new) in &self.replace {
            let location = format!("replace.{old}");
            reader.rule_id(&location, old, Use::Other);
            reader.rule_id(&location, new, Use::Other);
        }
        reader.rules(&self.rules, turn_on);
        for (index, over) in self.overrides.iter().enumerate() {
            reader.overrides(index, over);
        }
        if reader.found.is_empty() {
            Ok(reader.warnings.into_iter().collect())
        } else {
            Err(ConfigError::new(reader.found))
        }
    }
}

/// What reading the names of the rules of a configuration found: the errors, by location, and the
/// warnings, once each.
pub(crate) struct Reader<'a> {
    pub catalog: &'a Catalog,
    pub found: Vec<ConfigIssue>,
    pub warnings: BTreeSet<String>,
}

impl<'a> Reader<'a> {
    pub fn selectors(&mut self, key: &str, values: &[String], how: Use) {
        for (index, value) in values.iter().enumerate() {
            match Selector::parse(value).and_then(|selector| selector.check(self.catalog, how)) {
                Ok(warning) => self.warnings.extend(warning),
                Err(message) => self.found.push(issue(format!("{key}[{index}]"), message)),
            }
        }
    }

    fn rule_id(&mut self, location: &str, id: &str, how: Use) -> Option<&'a Rule> {
        match read_rule(self.catalog, id, how) {
            Ok(warning) => {
                self.warnings.extend(warning);
                self.catalog.resolve(id)
            }
            Err(message) => {
                self.found.push(issue(location, message));
                None
            }
        }
    }

    fn rules(&mut self, configured: &BTreeMap<String, RuleConfig>, turn_on: Use) {
        let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
        for (key, config) in configured {
            let location = format!("rules.{key}");
            let how = if config.enabled == Some(true) {
                turn_on
            } else {
                Use::Other
            };
            let Some(rule) = self.rule_id(&location, key, how) else {
                continue;
            };
            if let Some(first) = seen.insert(rule.id.as_str(), key) {
                self.found.push(issue(
                    &location,
                    format!("the same rule is configured twice, as `{first}` and as `{key}`"),
                ));
            }
            rule_config(&mut self.found, &location, rule, config);
        }
    }

    fn overrides(&mut self, index: usize, over: &Override) {
        let base = format!("overrides[{index}]");
        if over.files.is_empty() {
            self.found.push(issue(
                format!("{base}.files"),
                "must list at least one glob",
            ));
        }
        globs(&mut self.found, &format!("{base}.files"), &over.files);
        self.selectors(&format!("{base}.ignore"), &over.ignore, Use::Other);
        for id in over.severity.keys() {
            self.rule_id(&format!("{base}.severity.{id}"), id, Use::Other);
        }
    }
}

/// A directory of code is written as the directory it is, from the project root, with nothing
/// around it: a path that cannot match what the walk reports would leave the rules about layout
/// doing nothing. The ecosystems are the ones the linter knows.
fn code_dirs(found: &mut Vec<ConfigIssue>, configured: Option<&BTreeMap<String, Vec<String>>>) {
    for (name, values) in configured.into_iter().flatten() {
        if Ecosystem::parse(name).is_none() {
            let known: Vec<&str> = Ecosystem::ALL.iter().map(|e| e.name()).collect();
            found.push(issue(
                format!("code-dirs.{name}"),
                format!(
                    "`{name}` is not an ecosystem the linter knows; they are {}",
                    known.join(", ")
                ),
            ));
        }
        for (index, value) in values.iter().enumerate() {
            let plain = value
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..");
            if !plain {
                found.push(issue(
                    format!("code-dirs.{name}[{index}]"),
                    format!(
                        "`{value}` is not a plain directory: write it from the project root, like `backend/app` or `frontend/src`, with no leading or trailing `/`, `.` or `..`"
                    ),
                ));
            }
        }
    }
}

fn globs(found: &mut Vec<ConfigIssue>, key: &str, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        if let Err(error) = PathMatcher::check(value) {
            found.push(issue(
                format!("{key}[{index}]"),
                format!("invalid glob `{value}`: {error}"),
            ));
        }
    }
}

/// The globs and the options of `[rules.<name>]`.
fn rule_config(found: &mut Vec<ConfigIssue>, location: &str, rule: &Rule, config: &RuleConfig) {
    globs(
        found,
        &format!("{location}.ignores"),
        config.ignores.as_deref().unwrap_or_default(),
    );
    globs(
        found,
        &format!("{location}.extend-ignores"),
        &config.extend_ignores,
    );
    for (name, value) in &config.options {
        if let Err(message) = check_option(rule, name, value) {
            found.push(issue(format!("{location}.{name}"), message));
        }
    }
}

fn check_option(rule: &Rule, name: &str, value: &toml::Value) -> Result<(), String> {
    let Some(spec) = rule.options.get(name) else {
        let valid: Vec<&str> = rule.options.keys().map(String::as_str).collect();
        return Err(if valid.is_empty() {
            format!(
                "rule `{}` has no options, so `{name}` is not valid",
                rule.id
            )
        } else {
            format!(
                "rule `{}` has no option `{name}` (its options: {})",
                rule.id,
                valid.join(", ")
            )
        });
    };
    check_value(name, spec, value)
}

fn check_value(name: &str, spec: &OptionSpec, value: &toml::Value) -> Result<(), String> {
    match (spec.kind, value) {
        (OptionType::Int, toml::Value::Integer(n)) => match spec.min {
            Some(min) if *n < min => Err(format!("option `{name}` must be at least {min}")),
            _ => Ok(()),
        },
        (OptionType::Int, _) => Err(format!("option `{name}` must be an integer")),
        (OptionType::Bool, toml::Value::Boolean(_))
        | (OptionType::String, toml::Value::String(_)) => Ok(()),
        (OptionType::Bool, _) => Err(format!("option `{name}` must be a boolean")),
        (OptionType::String, _) => Err(format!("option `{name}` must be a string")),
        (OptionType::Strings, toml::Value::Array(items))
            if items.iter().all(toml::Value::is_str) =>
        {
            Ok(())
        }
        (OptionType::Strings, _) => Err(format!("option `{name}` must be a list of strings")),
    }
}
