use std::collections::BTreeMap;

use botica_catalog::{Catalog, OptionSpec, OptionType, Rule};

use crate::issue::{issue, ConfigError, ConfigIssue};
use crate::matcher::PathMatcher;
use crate::model::{Config, Override, RuleConfig};
use crate::selector::{check_rule, Selector};

impl Config {
    pub fn validate(&self, catalog: &Catalog) -> Result<(), ConfigError> {
        let mut found = Vec::new();
        selectors(&mut found, catalog, "select", &self.select);
        selectors(&mut found, catalog, "extend-select", &self.extend_select);
        selectors(&mut found, catalog, "ignore", &self.ignore);
        globs(&mut found, "extend-exclude", &self.extend_exclude);
        globs(&mut found, "external", &self.external);
        for (old, new) in &self.replace {
            let location = format!("replace.{old}");
            rule_id(&mut found, catalog, &location, old);
            rule_id(&mut found, catalog, &location, new);
        }
        rules(&mut found, catalog, &self.rules);
        for (index, over) in self.overrides.iter().enumerate() {
            overrides(&mut found, catalog, index, over);
        }
        if found.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::new(found))
        }
    }
}

fn selectors(found: &mut Vec<ConfigIssue>, catalog: &Catalog, key: &str, values: &[String]) {
    for (index, value) in values.iter().enumerate() {
        let verdict = Selector::parse(value).and_then(|selector| selector.check(catalog));
        if let Err(message) = verdict {
            found.push(issue(format!("{key}[{index}]"), message));
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

fn rule_id<'a>(
    found: &mut Vec<ConfigIssue>,
    catalog: &'a Catalog,
    location: &str,
    id: &str,
) -> Option<&'a Rule> {
    let rule = catalog.resolve(id);
    match check_rule(rule, id) {
        Ok(()) => rule,
        Err(message) => {
            found.push(issue(location, message));
            None
        }
    }
}

fn rules(
    found: &mut Vec<ConfigIssue>,
    catalog: &Catalog,
    configured: &BTreeMap<String, RuleConfig>,
) {
    let mut seen: BTreeMap<&str, &str> = BTreeMap::new();
    for (key, config) in configured {
        let location = format!("rules.{key}");
        let Some(rule) = rule_id(found, catalog, &location, key) else {
            continue;
        };
        if let Some(first) = seen.insert(rule.id.as_str(), key) {
            found.push(issue(
                &location,
                format!("the same rule is configured twice, as `{first}` and as `{key}`"),
            ));
        }
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
    }
}

fn overrides(found: &mut Vec<ConfigIssue>, catalog: &Catalog, index: usize, over: &Override) {
    let base = format!("overrides[{index}]");
    if over.files.is_empty() {
        found.push(issue(
            format!("{base}.files"),
            "must list at least one glob",
        ));
    }
    globs(found, &format!("{base}.files"), &over.files);
    selectors(found, catalog, &format!("{base}.ignore"), &over.ignore);
    for id in over.severity.keys() {
        rule_id(found, catalog, &format!("{base}.severity.{id}"), id);
    }
}
