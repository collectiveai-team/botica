use std::collections::BTreeMap;

use serde::de::DeserializeOwned;

use crate::issue::{issue, CatalogError, CatalogIssue};
use crate::model::{Family, FamilyFile, Rule};
use crate::validate::{check_across, check_family_code, check_rule, is_kebab_case};

pub const OWN_RULES_DIR: &str = "botica";

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
}

#[derive(Default)]
struct RuleEntry<'a> {
    toml: Option<&'a SourceFile>,
    readme: Option<&'a SourceFile>,
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
        })
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
            [dir, "family.toml"] => dirs.entry(dir).or_default().family = Some(file),
            [dir, slug, "rule.toml"] => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().toml = Some(file);
            }
            [dir, slug, "README.md"] => {
                dirs.entry(dir).or_default().rules.entry(slug).or_default().readme = Some(file);
            }
            _ => issues.push(issue(
                file.path.as_str(),
                "unexpected file: expected <family>/family.toml or <family>/<slug>/{rule.toml,README.md}",
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
            rule.family = family.map(|f| f.name.clone());
            rule.readme = readme.text.clone();
            rules.push((toml_file.path.clone(), rule));
        }
    }
    rules
}
