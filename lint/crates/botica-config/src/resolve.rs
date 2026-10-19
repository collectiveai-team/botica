use std::collections::BTreeMap;

use botica_catalog::{Catalog, Implementation, Rule, Scope, Severity};

use crate::entry::{entry, runs, severity_of, Entry, Reason};
use crate::issue::ConfigError;
use crate::matcher::PathMatcher;
use crate::model::{Config, RuleConfig, RuleSeverity};
use crate::selector::Selector;

#[derive(Debug)]
pub struct FileRule<'r> {
    pub rule: &'r Rule,
    pub severity: Severity,
    /// The severity the configuration names for this rule at this path (`rules.<id>` or an
    /// override). `None` leaves each finding with the severity its rule gave it.
    pub configured: Option<Severity>,
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
    exclude: PathMatcher,
    include: PathMatcher,
}

pub(crate) type Selectors = Vec<(String, Selector)>;

pub(crate) fn selectors_of(values: &[String]) -> Selectors {
    values
        .iter()
        .filter_map(|text| {
            Selector::parse(text)
                .ok()
                .map(|selector| (text.clone(), selector))
        })
        .collect()
}

fn matcher_of(patterns: &[String], key: &str) -> Result<PathMatcher, ConfigError> {
    PathMatcher::new(patterns).map_err(|errors| {
        let detail = errors
            .iter()
            .map(|(_, message)| message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        ConfigError::single(key, detail)
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
            external: matcher_of(&config.external, "external")?,
            exclude: matcher_of(&config.extend_exclude, "extend-exclude")?,
            include: matcher_of(&config.extend_include, "extend-include")?,
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

    /// Keeps active only the rules of this scope (`None` keeps them all). A rule that was not
    /// active keeps the reason it had, and a rule with no scope (`botica lint` does not run it: a
    /// judge or another tool does) stays as it is.
    pub fn only_scope(mut self, scope: Option<Scope>) -> Self {
        let Some(scope) = scope else {
            return self;
        };
        let name = match scope {
            Scope::File => "file",
            Scope::Project => "project",
        };
        for entry in self.entries.iter_mut().filter(|entry| entry.active) {
            if entry.rule.scope.is_some_and(|own| own != scope) {
                entry.active = false;
                entry.reason = Reason::OutOfScope(name.to_owned());
            }
        }
        self
    }

    /// The code of each catalog of the project, which the codes of its rules start with.
    pub fn project_codes(&self) -> Vec<&str> {
        self.catalog.project_codes()
    }

    /// Whether a suppression may name `id` without a warning although no rule has it.
    pub fn is_external(&self, id: &str) -> bool {
        self.external.is_match(id)
    }

    /// Whether `extend-exclude` names anything, so a walk has something to check.
    pub fn has_exclusions(&self) -> bool {
        !self.exclude.is_empty()
    }

    /// Whether a walk skips this entry (a directory is skipped with everything in it). Paths are
    /// from the project root.
    pub fn excludes_entry(&self, path: &str, is_dir: bool) -> bool {
        self.exclude.is_match_entry(path, is_dir)
    }

    /// Whether a path given by name is excluded, itself or through a directory above it.
    pub fn excludes_path(&self, path: &str) -> bool {
        self.exclude.is_match(path)
    }

    /// Whether `extend-include` names anything, so a walk has something to look for beyond what
    /// it reaches by itself.
    pub fn has_inclusions(&self) -> bool {
        !self.include.is_empty()
    }

    /// Whether `extend-include` asks for this path, itself or through a directory above it.
    /// Paths are from the project root.
    pub fn includes_path(&self, path: &str) -> bool {
        self.include.is_match(path)
    }

    /// Whether a walk looking for what `extend-include` asks for should enter this directory.
    pub fn include_may_reach(&self, dir: &str) -> bool {
        self.include.may_match_below(dir)
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
        self.rules_at(path, language)
    }

    /// Like `rules_for` for a finding that has no language of its own (a path that cannot be
    /// read): the `languages` of the configuration do not restrict it.
    pub fn rules_for_path<'r>(&'r self, path: &str) -> Vec<FileRule<'r>> {
        self.rules_at(path, None)
    }

    fn rules_at<'r>(&'r self, path: &str, language: Option<&str>) -> Vec<FileRule<'r>> {
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
                    configured: entry.configured_severity,
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
                            file_rule.configured = Some(level);
                        }
                    }
                    None => found.retain(|r| r.rule.id != *id),
                }
            }
        }
        found
    }
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
