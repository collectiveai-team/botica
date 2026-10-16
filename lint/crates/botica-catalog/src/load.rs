use std::collections::BTreeMap;

use serde::de::DeserializeOwned;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::model::{Event, Family, FamilyFile, HistoryFile, Lifecycle, Rule};
use crate::origin::{Origin, HEADER};
use crate::readme::check_readme;
use crate::sources::{attach_configs, attach_scripts, attach_sources, is_script};
use crate::validate::{check_across, check_family_code, check_history, check_rule, is_kebab_case};

pub const OWN_RULES_DIR: &str = "botica";
pub const HISTORY_FILE: &str = "history.toml";
pub const CATALOG_FILE: &str = "catalog.toml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Catalog {
    families: Vec<Family>,
    rules: Vec<Rule>,
    index: BTreeMap<String, usize>,
    history: Vec<Event>,
}

/// What one load of files produced: the families, the rules with the path of each `rule.toml`,
/// and the history, once the files have passed every check.
pub(crate) struct Parts {
    pub families: Vec<Family>,
    pub rules: Vec<(String, Rule)>,
    pub history: Vec<Event>,
}

#[derive(Default)]
struct RuleEntry<'a> {
    toml: Option<&'a SourceFile>,
    readme: Option<&'a SourceFile>,
    /// The YAML files of the implementations.
    sources: Vec<&'a SourceFile>,
    /// The fragments of configuration of the external implementations, `config/<file>.toml`.
    configs: Vec<&'a SourceFile>,
    /// The scripts of the folder: the ones the extensions name and the modules they import.
    scripts: Vec<&'a SourceFile>,
}

#[derive(Default)]
struct DirEntry<'a> {
    family: Option<&'a SourceFile>,
    rules: BTreeMap<&'a str, RuleEntry<'a>>,
}

impl Catalog {
    pub fn from_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        let mut issues = Vec::new();
        let parts = load_parts(files, Origin::BuiltIn, &mut issues);
        if !issues.is_empty() {
            return Err(CatalogError { issues });
        }
        let mut catalog = Self {
            families: Vec::new(),
            rules: Vec::new(),
            index: BTreeMap::new(),
            history: Vec::new(),
        };
        catalog.append(parts);
        Ok(catalog)
    }

    /// Adds what a load produced after what the catalog holds: its families, its rules in the
    /// order of their codes, and its history.
    pub(crate) fn append(&mut self, parts: Parts) {
        self.families.extend(parts.families);
        for (_, rule) in parts.rules {
            let position = self.rules.len();
            self.index.insert(rule.id.clone(), position);
            self.index.insert(rule.code.clone(), position);
            for alias in &rule.aliases {
                self.index.insert(alias.clone(), position);
            }
            self.rules.push(rule);
        }
        self.history.extend(parts.history);
    }

    /// Whether `key` is the id, the code or an alias of a rule.
    pub(crate) fn knows(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    /// When `rule` was released, from the history of the catalog.
    pub fn lifecycle(&self, rule: &Rule) -> Lifecycle {
        let mut events = self.history.iter().filter(|event| event.rule == rule.id);
        let added = events.next().map(|event| event.version.clone());
        let since = self
            .history
            .iter()
            .rfind(|event| event.rule == rule.id && event.status == rule.status)
            .map(|event| event.version.clone());
        Lifecycle { added, since }
    }

    pub fn families(&self) -> &[Family] {
        &self.families
    }

    /// The code of each catalog of a project that has a family, in the order of the catalogs.
    pub fn project_codes(&self) -> Vec<&str> {
        let mut codes: Vec<&str> = Vec::new();
        for family in self.families.iter().filter(|f| f.catalog.is_some()) {
            if !codes.contains(&family.header.as_str()) {
                codes.push(&family.header);
            }
        }
        codes
    }

    /// Whether `letters` is what the codes of a catalog start with: `BT` for the built-in
    /// catalog, or the code of a catalog of a project that has a family.
    pub fn has_header(&self, letters: &str) -> bool {
        letters == HEADER || self.families.iter().any(|family| family.header == letters)
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn resolve(&self, key: &str) -> Option<&Rule> {
        self.index.get(key).map(|position| &self.rules[*position])
    }
}

/// Loads the files of one catalog, with paths from its folder, and reports what is wrong.
pub(crate) fn load_parts(
    files: &[SourceFile],
    origin: Origin<'_>,
    issues: &mut Vec<CatalogIssue>,
) -> Parts {
    let dirs = scan(files, origin, issues);
    let families = load_families(&dirs, origin, issues);
    let mut rules = load_rules(&dirs, &families, origin, issues);
    check_across(&families, &rules, origin, issues);
    let history = if origin.is_built_in() {
        load_history(files, &rules, issues)
    } else {
        Vec::new()
    };
    rules.sort_by(|(_, a), (_, b)| a.code.cmp(&b.code));
    Parts {
        families,
        rules,
        history,
    }
}

fn scan<'a>(
    files: &'a [SourceFile],
    origin: Origin<'_>,
    issues: &mut Vec<CatalogIssue>,
) -> BTreeMap<&'a str, DirEntry<'a>> {
    let mut dirs: BTreeMap<&str, DirEntry> = BTreeMap::new();
    for file in files {
        let parts: Vec<&str> = file.path.split('/').collect();
        match parts.as_slice() {
            [HISTORY_FILE] if origin.is_built_in() => {}
            [dir, "family.toml"] => dirs.entry(dir).or_default().family = Some(file),
            [dir, slug, "rule.toml"] => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().toml = Some(file);
            }
            [dir, slug, "README.md"] => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().readme = Some(file);
            }
            [dir, slug, name] if is_script(name) => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().scripts.push(file);
            }
            [dir, slug, name] if name.ends_with(".yml") => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().sources.push(file);
            }
            [dir, slug, "config", name] if name.ends_with(".toml") => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().configs.push(file);
            }
            _ => issues.push(issue(
                file.path.as_str(),
                "unexpected file: expected history.toml, <family>/family.toml or <family>/<slug>/{rule.toml,README.md,<implementation>.yml,<script>.py|.js|.mjs,config/<file>.toml}",
            )),
        }
    }
    dirs
}

fn parse<T: DeserializeOwned>(file: &SourceFile, issues: &mut Vec<CatalogIssue>) -> Option<T> {
    match toml::from_str(&file.text) {
        Ok(value) => Some(value),
        Err(error) => {
            issues.push(issue(file.path.as_str(), error.message().to_owned()));
            None
        }
    }
}

fn load_history(
    files: &[SourceFile],
    rules: &[(String, Rule)],
    issues: &mut Vec<CatalogIssue>,
) -> Vec<Event> {
    let Some(file) = files.iter().find(|file| file.path == HISTORY_FILE) else {
        return Vec::new();
    };
    let Some(parsed) = parse::<HistoryFile>(file, issues) else {
        return Vec::new();
    };
    check_history(&parsed.event, rules, issues);
    parsed.event
}

fn load_families(
    dirs: &BTreeMap<&str, DirEntry<'_>>,
    origin: Origin<'_>,
    issues: &mut Vec<CatalogIssue>,
) -> Vec<Family> {
    let mut families = Vec::new();
    // Only the built-in catalog has a folder for the rules of the linter itself.
    let own_dir = origin.is_built_in().then_some(OWN_RULES_DIR);
    for (dir, entry) in dirs {
        let Some(file) = entry.family else {
            if Some(*dir) != own_dir {
                let exempt = own_dir
                    .map(|own| format!(" (only `{own}/` is exempt)"))
                    .unwrap_or_default();
                issues.push(issue(
                    format!("{dir}/family.toml"),
                    format!("folder `{dir}` has no family.toml{exempt}"),
                ));
            }
            continue;
        };
        if Some(*dir) == own_dir {
            issues.push(issue(
                file.path.as_str(),
                format!("`{OWN_RULES_DIR}/` holds the linter's own rules and takes no family.toml"),
            ));
            continue;
        }
        if !is_kebab_case(dir) {
            issues.push(issue(
                file.path.as_str(),
                format!("folder `{dir}` must be a kebab-case family name"),
            ));
        }
        let Some(parsed) = parse::<FamilyFile>(file, issues) else {
            continue;
        };
        if let Some(message) = check_family_code(&parsed.code) {
            issues.push(issue(file.path.as_str(), message));
        }
        families.push(Family {
            name: (*dir).to_owned(),
            code: parsed.code,
            description: parsed.description,
            header: origin.header().to_owned(),
            catalog: origin.catalog(),
        });
    }
    families
}

fn load_rules(
    dirs: &BTreeMap<&str, DirEntry<'_>>,
    families: &[Family],
    origin: Origin<'_>,
    issues: &mut Vec<CatalogIssue>,
) -> Vec<(String, Rule)> {
    let mut rules = Vec::new();
    for (dir, entry) in dirs {
        let family = families.iter().find(|f| f.name == *dir);
        for (slug, files) in &entry.rules {
            let base = format!("{dir}/{slug}");
            let Some(toml_file) = files.toml else {
                issues.push(issue(format!("{base}/rule.toml"), "missing rule.toml"));
                continue;
            };
            let Some(readme) = files.readme else {
                issues.push(issue(format!("{base}/README.md"), "missing README.md"));
                continue;
            };
            let Some(mut rule) = parse::<Rule>(toml_file, issues) else {
                continue;
            };
            issues.extend(check_rule(&toml_file.path, slug, &rule, family, origin));
            attach_sources(&base, &mut rule, &files.sources, issues);
            attach_configs(&base, &mut rule, &files.configs, issues);
            attach_scripts(&base, &mut rule, &files.scripts, issues);
            rule.family = family.map(|f| f.name.clone());
            rule.catalog = origin.catalog();
            rule.readme = readme.text.clone();
            issues.extend(check_readme(&readme.path, &rule));
            rules.push((toml_file.path.clone(), rule));
        }
    }
    rules
}
