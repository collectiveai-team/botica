use std::collections::BTreeMap;

use crate::issue::{issue, CatalogIssue};
use crate::model::{Family, Kind, Rule};

pub(crate) const HEADER: &str = "BT";

const RESERVED_OPTION_NAMES: [&str; 4] = ["severity", "enabled", "ignores", "extend-ignores"];

pub fn is_kebab_case(text: &str) -> bool {
    !text.is_empty()
        && text.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

pub(crate) fn check_family_code(code: &str) -> Option<String> {
    let valid = !code.is_empty() && code.bytes().all(|b| b.is_ascii_uppercase());
    (!valid).then(|| {
        format!("family code `{code}` must be one or more letters A-Z (a digit or a separator breaks `noqa` parsing)")
    })
}

fn check_rule_code(code: &str, family: Option<&Family>) -> Option<String> {
    let prefix = format!("{HEADER}{}", family.map_or("", |f| f.code.as_str()));
    let digits = code.strip_prefix(prefix.as_str());
    let valid = digits.is_some_and(|d| d.len() == 3 && d.bytes().all(|b| b.is_ascii_digit()));
    (!valid).then(|| format!("code `{code}` must be `{prefix}` followed by exactly 3 digits"))
}

fn check_since(since: &str) -> bool {
    let parts: Vec<&str> = since.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

fn check_ces(ces: &str) -> bool {
    ces.strip_prefix("CES-")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

pub(crate) fn check_rule(
    path: &str,
    folder_slug: &str,
    rule: &Rule,
    family: Option<&Family>,
) -> Vec<CatalogIssue> {
    let mut found = Vec::new();
    let mut add = |message: String| found.push(issue(path, message));

    if !is_kebab_case(folder_slug) {
        add(format!("folder `{folder_slug}` must be a kebab-case slug"));
    }
    if rule.id != folder_slug {
        add(format!(
            "id `{}` must equal its folder name `{folder_slug}`",
            rule.id
        ));
    }
    if let Some(message) = check_rule_code(&rule.code, family) {
        add(message);
    }
    if let Some(ces) = &rule.ces {
        if !check_ces(ces) {
            add(format!("ces `{ces}` must look like `CES-71`"));
        }
    }
    for name in rule.options.keys() {
        if RESERVED_OPTION_NAMES.contains(&name.as_str()) {
            add(format!(
                "option `{name}` is a key every rule has in the configuration: pick another name"
            ));
        } else if !is_kebab_case(name) {
            add(format!("option `{name}` must be kebab-case"));
        }
    }
    if !check_since(&rule.since) {
        add(format!("since `{}` must look like `0.1.0`", rule.since));
    }
    if rule.implementations.is_empty() {
        add("a rule needs at least one implementation".to_owned());
    }
    for (index, implementation) in rule.implementations.iter().enumerate() {
        if implementation.languages.is_empty() {
            add(format!(
                "implementation {index}: languages is required and cannot be empty"
            ));
        }
        let is_extension = implementation.kind == Kind::Extension;
        if is_extension && implementation.runtime.is_none() {
            add(format!(
                "implementation {index}: an extension needs a runtime"
            ));
        }
        if !is_extension && implementation.runtime.is_some() {
            add(format!(
                "implementation {index}: runtime only applies to extensions"
            ));
        }
    }
    found
}

pub(crate) fn check_across(
    families: &[Family],
    rules: &[(String, Rule)],
    found: &mut Vec<CatalogIssue>,
) {
    let mut family_codes: BTreeMap<&str, &str> = BTreeMap::new();
    for family in families {
        if let Some(first) = family_codes.insert(&family.code, &family.name) {
            found.push(issue(
                format!("{}/family.toml", family.name),
                format!("family code `{}` is already used by `{first}`", family.code),
            ));
        }
    }

    for short in families {
        for long in families {
            if short.code != long.code && long.code.starts_with(&short.code) {
                found.push(issue(
                    format!("{}/family.toml", short.name),
                    format!(
                        "family code `{}` is a text prefix of `{}` (`{}`): selecting `{HEADER}{}` would be ambiguous",
                        short.code, long.code, long.name, short.code
                    ),
                ));
            }
        }
    }

    let mut slugs: BTreeMap<&str, &str> = BTreeMap::new();
    let mut codes: BTreeMap<&str, &str> = BTreeMap::new();
    for (path, rule) in rules {
        if let Some(first) = slugs.insert(&rule.id, path) {
            found.push(issue(
                path.as_str(),
                format!(
                    "slug `{}` is declared in more than one place (also `{first}`)",
                    rule.id
                ),
            ));
        }
        if let Some(first) = codes.insert(&rule.code, &rule.id) {
            found.push(issue(
                path.as_str(),
                format!("code `{}` is already used by `{first}`", rule.code),
            ));
        }
    }

    let mut aliases: BTreeMap<&str, &str> = BTreeMap::new();
    for (path, rule) in rules {
        for alias in &rule.aliases {
            if slugs.contains_key(alias.as_str()) || codes.contains_key(alias.as_str()) {
                found.push(issue(
                    path.as_str(),
                    format!("alias `{alias}` collides with the slug or code of a rule"),
                ));
            } else if let Some(first) = aliases.insert(alias, &rule.id) {
                found.push(issue(
                    path.as_str(),
                    format!("alias `{alias}` is already an alias of `{first}`"),
                ));
            }
        }
    }
}
