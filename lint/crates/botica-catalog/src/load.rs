use std::collections::BTreeMap;

use serde::de::DeserializeOwned;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::model::{Event, Family, FamilyFile, HistoryFile, Kind, Lifecycle, Rule};
use crate::readme::check_readme;
use crate::validate::{check_across, check_family_code, check_history, check_rule, is_kebab_case};

pub const OWN_RULES_DIR: &str = "botica";
pub const HISTORY_FILE: &str = "history.toml";

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

#[derive(Default)]
struct RuleEntry<'a> {
    toml: Option<&'a SourceFile>,
    readme: Option<&'a SourceFile>,
    /// The YAML files of the implementations.
    sources: Vec<&'a SourceFile>,
    /// The fragments of configuration of the external implementations, `config/<file>.toml`.
    configs: Vec<&'a SourceFile>,
}

#[derive(Default)]
struct DirEntry<'a> {
    family: Option<&'a SourceFile>,
    rules: BTreeMap<&'a str, RuleEntry<'a>>,
}

impl Catalog {
    pub fn from_files(files: &[SourceFile]) -> Result<Self, CatalogError> {
        let mut issues = Vec::new();
        let dirs = scan(files, &mut issues);
        let families = load_families(&dirs, &mut issues);
        let rules = load_rules(&dirs, &families, &mut issues);
        check_across(&families, &rules, &mut issues);
        let history = load_history(files, &rules, &mut issues);
        if !issues.is_empty() {
            return Err(CatalogError { issues });
        }

        let mut rules: Vec<Rule> = rules.into_iter().map(|(_, rule)| rule).collect();
        rules.sort_by(|a, b| a.code.cmp(&b.code));
        let mut index = BTreeMap::new();
        for (position, rule) in rules.iter().enumerate() {
            index.insert(rule.id.clone(), position);
            index.insert(rule.code.clone(), position);
            for alias in &rule.aliases {
                index.insert(alias.clone(), position);
            }
        }
        Ok(Self {
            families,
            rules,
            index,
            history,
        })
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

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn resolve(&self, key: &str) -> Option<&Rule> {
        self.index.get(key).map(|position| &self.rules[*position])
    }
}

fn scan<'a>(
    files: &'a [SourceFile],
    issues: &mut Vec<CatalogIssue>,
) -> BTreeMap<&'a str, DirEntry<'a>> {
    let mut dirs: BTreeMap<&str, DirEntry> = BTreeMap::new();
    for file in files {
        let parts: Vec<&str> = file.path.split('/').collect();
        match parts.as_slice() {
            [HISTORY_FILE] => {}
            [dir, "family.toml"] => dirs.entry(dir).or_default().family = Some(file),
            [dir, slug, "rule.toml"] => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().toml = Some(file);
            }
            [dir, slug, "README.md"] => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().readme = Some(file);
            }
            [dir, slug, name] if name.ends_with(".yml") => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().sources.push(file);
            }
            [dir, slug, "config", name] if name.ends_with(".toml") => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().configs.push(file);
            }
            _ => issues.push(issue(
                file.path.as_str(),
                "unexpected file: expected history.toml, <family>/family.toml or <family>/<slug>/{rule.toml,README.md,<implementation>.yml,config/<file>.toml}",
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
    issues: &mut Vec<CatalogIssue>,
) -> Vec<Family> {
    let mut families = Vec::new();
    for (dir, entry) in dirs {
        let Some(file) = entry.family else {
            if *dir != OWN_RULES_DIR {
                issues.push(issue(
                    format!("{dir}/family.toml"),
                    format!(
                        "folder `{dir}` has no family.toml (only `{OWN_RULES_DIR}/` is exempt)"
                    ),
                ));
            }
            continue;
        };
        if *dir == OWN_RULES_DIR {
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
        });
    }
    families
}

fn load_rules(
    dirs: &BTreeMap<&str, DirEntry<'_>>,
    families: &[Family],
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
            issues.extend(check_rule(&toml_file.path, slug, &rule, family));
            attach_sources(&base, &mut rule, &files.sources, issues);
            attach_configs(&base, &mut rule, &files.configs, issues);
            rule.family = family.map(|f| f.name.clone());
            rule.readme = readme.text.clone();
            issues.extend(check_readme(&readme.path, &rule));
            rules.push((toml_file.path.clone(), rule));
        }
    }
    rules
}

/// What is wrong with the top-level `id:` of an ast-grep YAML, if anything: diagnostics carry
/// that id, so it has to be the slug the catalog, the selection and the suppressions know.
fn id_problem(yaml: &str, slug: &str) -> Option<String> {
    let found: Vec<&str> = yaml
        .lines()
        .filter_map(|line| line.strip_prefix("id:"))
        .map(|id| id.trim().trim_matches(['"', '\'']))
        .collect();
    (found != [slug]).then(|| {
        if found.is_empty() {
            format!("the YAML must declare `id: {slug}`")
        } else {
            format!(
                "the YAML must declare only `id: {slug}`, found `{}`",
                found.join("`, `")
            )
        }
    })
}

/// Gives each ast-grep implementation the text of the file it names, and rejects the files that
/// are named by nobody, the ones that are missing and the implementations that name none.
fn attach_sources(
    base: &str,
    rule: &mut Rule,
    sources: &[&SourceFile],
    issues: &mut Vec<CatalogIssue>,
) {
    let mut named = Vec::new();
    for implementation in &mut rule.implementations {
        if implementation.kind != Kind::AstGrep {
            continue;
        }
        let Some(name) = implementation.file.clone() else {
            issues.push(issue(
                format!("{base}/rule.toml"),
                "an ast-grep implementation must name its YAML with `file`",
            ));
            continue;
        };
        let path = format!("{base}/{name}");
        match sources.iter().find(|source| source.path == path) {
            Some(source) => {
                if let Some(problem) = id_problem(&source.text, &rule.id) {
                    issues.push(issue(path, problem));
                }
                implementation.source = Some(source.text.clone());
            }
            None => issues.push(issue(
                path,
                format!(
                    "`{name}` is named by an implementation of `{}` and is missing",
                    rule.id
                ),
            )),
        }
        named.push(name);
    }
    reject_unnamed(base, sources, &named, "file", issues);
}

/// Gives each external implementation the text of the fragment that `config` names, and rejects
/// the fragments that are named by nobody, the ones that are missing and those that are not TOML.
fn attach_configs(
    base: &str,
    rule: &mut Rule,
    configs: &[&SourceFile],
    issues: &mut Vec<CatalogIssue>,
) {
    let mut named = Vec::new();
    for implementation in &mut rule.implementations {
        let Some(name) = implementation.config.clone() else {
            continue;
        };
        let path = format!("{base}/{name}");
        match configs.iter().find(|config| config.path == path) {
            Some(config) => {
                if let Err(error) = config.text.parse::<toml::Table>() {
                    issues.push(issue(path, error.message().to_owned()));
                }
                implementation.source = Some(config.text.clone());
            }
            None => issues.push(issue(
                path,
                format!(
                    "`{name}` is named by an implementation of `{}` and is missing",
                    rule.id
                ),
            )),
        }
        named.push(name);
    }
    reject_unnamed(base, configs, &named, "config", issues);
}

fn reject_unnamed(
    base: &str,
    files: &[&SourceFile],
    named: &[String],
    field: &str,
    issues: &mut Vec<CatalogIssue>,
) {
    for file in files {
        let used = named
            .iter()
            .any(|name| file.path == format!("{base}/{name}"));
        if !used {
            issues.push(issue(
                file.path.as_str(),
                format!("no implementation names this file with `{field}`"),
            ));
        }
    }
}
