use std::collections::BTreeMap;

use crate::issue::{issue, CatalogIssue};
use crate::model::{Family, Rule};
use crate::origin::Origin;

/// The checks that compare the families and the rules of a catalog with one another.
pub(crate) fn check_across(
    families: &[Family],
    rules: &[(String, Rule)],
    outer: &[Rule],
    origin: Origin<'_>,
    found: &mut Vec<CatalogIssue>,
) {
    check_family_codes(families, found);
    check_family_prefixes(families, origin, found);
    let (slugs, codes) = check_rule_names(rules, found);
    check_aliases(rules, &slugs, &codes, found);
    check_covered_by(rules, outer, found);
}

fn check_family_codes(families: &[Family], found: &mut Vec<CatalogIssue>) {
    let mut family_codes: BTreeMap<&str, &str> = BTreeMap::new();
    for family in families {
        if let Some(first) = family_codes.insert(&family.code, &family.name) {
            found.push(issue(
                format!("{}/family.toml", family.name),
                format!("family code `{}` is already used by `{first}`", family.code),
            ));
        }
    }
}

fn check_family_prefixes(families: &[Family], origin: Origin<'_>, found: &mut Vec<CatalogIssue>) {
    for short in families {
        for long in families {
            if short.code != long.code && long.code.starts_with(&short.code) {
                found.push(issue(
                    format!("{}/family.toml", short.name),
                    format!(
                        "family code `{}` is a text prefix of `{}` (`{}`): selecting `{}{}` would be ambiguous",
                        short.code,
                        long.code,
                        long.name,
                        origin.header(),
                        short.code
                    ),
                ));
            }
        }
    }
}

/// Reports a slug or a code that two rules share, and returns each slug with the path of its
/// rule and each code with the slug of its rule.
fn check_rule_names<'a>(
    rules: &'a [(String, Rule)],
    found: &mut Vec<CatalogIssue>,
) -> (BTreeMap<&'a str, &'a str>, BTreeMap<&'a str, &'a str>) {
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
    (slugs, codes)
}

fn check_aliases(
    rules: &[(String, Rule)],
    slugs: &BTreeMap<&str, &str>,
    codes: &BTreeMap<&str, &str>,
    found: &mut Vec<CatalogIssue>,
) {
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

/// `covered-by` names the deterministic rules that check part of a judgment rule: those of its
/// catalog and, for a catalog of a project, those of the catalogs loaded before it (`outer`).
fn check_covered_by(rules: &[(String, Rule)], outer: &[Rule], found: &mut Vec<CatalogIssue>) {
    for (path, rule) in rules {
        let covered = rule
            .implementations
            .iter()
            .flat_map(|implementation| &implementation.covered_by);
        for slug in covered {
            let named = rules
                .iter()
                .map(|(_, other)| other)
                .chain(outer)
                .find(|other| other.id == *slug);
            let message = match named {
                None => format!("covered-by `{slug}` is not a rule of the catalog"),
                Some(other) if other.is_judgment() => {
                    format!("covered-by `{slug}` is a judgment rule")
                }
                Some(_) => continue,
            };
            found.push(issue(path.as_str(), message));
        }
    }
}
