use std::collections::BTreeMap;

use crate::extension::check_extension;
use crate::issue::{issue, CatalogIssue};
use crate::model::{Event, Family, Implementation, Kind, Rule, Status};
use crate::origin::{check_origin, Origin};
use crate::question::check_question;

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

fn check_rule_code(code: &str, header: &str, family: Option<&Family>) -> Option<String> {
    let prefix = format!("{header}{}", family.map_or("", |f| f.code.as_str()));
    let digits = code.strip_prefix(prefix.as_str());
    let valid = digits.is_some_and(|d| d.len() == 3 && d.bytes().all(|b| b.is_ascii_digit()));
    (!valid).then(|| format!("code `{code}` must be `{prefix}` followed by exactly 3 digits"))
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let parts: Vec<&str> = version.split('.').collect();
    let numbers: Vec<u64> = parts
        .iter()
        .filter(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|p| p.parse().ok())
        .collect();
    match (parts.len(), numbers.as_slice()) {
        (3, [major, minor, patch]) => Some((*major, *minor, *patch)),
        _ => None,
    }
}

pub(crate) fn check_history(
    events: &[Event],
    rules: &[(String, Rule)],
    found: &mut Vec<CatalogIssue>,
) {
    let mut last_version: Option<((u64, u64, u64), &str)> = None;
    let mut last_status: BTreeMap<&str, Status> = BTreeMap::new();
    for event in events {
        let mut add = |message: String| found.push(issue("history.toml", message));
        if !rules.iter().any(|(_, rule)| rule.id == event.rule) {
            add(format!("`{}` is not a rule of the catalog", event.rule));
        }
        let Some(parsed) = parse_version(&event.version) else {
            add(format!(
                "version `{}` must look like `0.1.0`",
                event.version
            ));
            continue;
        };
        if let Some((previous, text)) = last_version {
            if parsed < previous {
                add(format!(
                    "version `{}` is older than `{text}`, the one of the event before: events are appended in release order",
                    event.version
                ));
            }
        }
        last_version = Some((parsed, &event.version));
        if last_status.insert(&event.rule, event.status) == Some(event.status) {
            add(format!(
                "`{}` is already {:?} in `{}`: an event has to change its status",
                event.rule, event.status, event.version
            ));
        }
    }
}

fn check_standard(standard: &str) -> bool {
    standard.split_once('-').is_some_and(|(set, number)| {
        check_family_code(set).is_none()
            && !number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit())
    })
}

/// The fields of the root of `rule.toml` that only a deterministic rule has, and whether each is
/// set.
fn deterministic_fields(rule: &Rule) -> [(&'static str, bool); 4] {
    [
        ("scope", rule.scope.is_some()),
        ("needs", rule.needs.is_some()),
        ("run-on", rule.run_on.is_some()),
        ("prefilter", rule.prefilter.is_some()),
    ]
}

fn check_deterministic(rule: &Rule, add: &mut impl FnMut(String)) {
    if rule.is_external_only() {
        for (field, _) in deterministic_fields(rule)
            .into_iter()
            .filter(|(_, set)| *set)
        {
            add(format!(
                "`{field}` applies to a rule that `botica lint` runs, and another tool enforces \
                 this one"
            ));
        }
    } else {
        for (field, set) in deterministic_fields(rule).into_iter().take(3) {
            if !set {
                add(format!("a deterministic rule needs `{field}`"));
            }
        }
    }
    for (index, implementation) in rule.implementations.iter().enumerate() {
        let judgment_fields = [
            ("when", implementation.when.is_some()),
            ("question", implementation.question.is_some()),
            ("covered-by", !implementation.covered_by.is_empty()),
            (
                "why-not-deterministic",
                implementation.why_not_deterministic.is_some(),
            ),
        ];
        for (field, _) in judgment_fields.into_iter().filter(|(_, set)| *set) {
            add(format!(
                "implementation {index}: `{field}` applies to a judgment implementation only"
            ));
        }
    }
}

fn check_judgment(rule: &Rule, add: &mut impl FnMut(String)) {
    for (field, _) in deterministic_fields(rule)
        .into_iter()
        .filter(|(_, set)| *set)
    {
        add(format!(
            "`{field}` applies to a deterministic rule, and this one is judgment"
        ));
    }
    if rule.implementations.len() > 1 {
        add("a judgment rule has one implementation".to_owned());
    }
    let Some((index, implementation)) = rule
        .implementations
        .iter()
        .enumerate()
        .find(|(_, implementation)| implementation.kind == Kind::Judgment)
    else {
        return;
    };
    let why = implementation
        .why_not_deterministic
        .as_deref()
        .is_some_and(|text| !text.trim().is_empty());
    for (field, set) in [
        ("when", implementation.when.is_some()),
        ("question", implementation.question.is_some()),
        ("why-not-deterministic", why),
    ] {
        if !set {
            add(format!("a judgment implementation needs `{field}`"));
        }
    }
    if let Some(question) = &implementation.question {
        for message in check_question(question) {
            add(format!("implementation {index}: question: {message}"));
        }
    }
}

pub(crate) fn check_rule(
    path: &str,
    folder_slug: &str,
    rule: &Rule,
    family: Option<&Family>,
    origin: Origin<'_>,
) -> Vec<CatalogIssue> {
    let mut found = Vec::new();
    let mut add = |message: String| found.push(issue(path, message));

    if !is_kebab_case(folder_slug) {
        add(format!("folder `{folder_slug}` must be a kebab-case slug"));
    }
    let expected = origin.id_of(folder_slug);
    if rule.id != expected {
        add(if origin.is_built_in() {
            format!(
                "id `{}` must equal its folder name `{folder_slug}`",
                rule.id
            )
        } else {
            format!(
                "id `{}` must equal `{expected}`: the name of its catalog and its folder name",
                rule.id
            )
        });
    }
    if let Some(message) = check_rule_code(&rule.code, origin.header(), family) {
        add(message);
    }
    if let Some(standard) = &rule.standard {
        if !check_standard(standard) {
            add(format!(
                "standard `{standard}` must look like `CES-71`: letters A-Z, a dash and a number"
            ));
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
    if rule.implementations.is_empty() {
        add("a rule needs at least one implementation".to_owned());
    }
    if rule.is_judgment() {
        check_judgment(rule, &mut add);
    } else {
        check_deterministic(rule, &mut add);
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
        check_external(index, implementation, &mut add);
        check_extension(index, implementation, &mut add);
        check_origin(index, rule, implementation, origin, &mut add);
    }
    found
}

/// What an external implementation needs, and the fields that only it may have.
fn check_external(index: usize, implementation: &Implementation, add: &mut impl FnMut(String)) {
    let fields = [
        ("tool", implementation.tool.is_some()),
        ("docs", implementation.docs.is_some()),
        ("codes", !implementation.codes.is_empty()),
        ("config", implementation.config.is_some()),
    ];
    if implementation.kind != Kind::External {
        for (field, _) in fields.into_iter().filter(|(_, set)| *set) {
            add(format!(
                "implementation {index}: `{field}` applies to an external implementation only"
            ));
        }
        return;
    }
    for (field, set) in fields.into_iter().take(2) {
        if !set {
            add(format!("an external implementation needs `{field}`"));
        }
    }
    if let Some(config) = &implementation.config {
        let valid = implementation.target().is_some_and(|name| {
            name.strip_suffix(".toml")
                .is_some_and(|stem| !stem.is_empty() && !stem.contains('/'))
        });
        if !valid {
            add(format!(
                "config `{config}` must be `config/<file>.toml`, the file of the project it goes into"
            ));
        }
    }
}

pub(crate) fn check_across(
    families: &[Family],
    rules: &[(String, Rule)],
    origin: Origin<'_>,
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
    check_covered_by(rules, found);
}

/// `covered-by` names the deterministic rules that check part of a judgment rule.
fn check_covered_by(rules: &[(String, Rule)], found: &mut Vec<CatalogIssue>) {
    for (path, rule) in rules {
        let covered = rule
            .implementations
            .iter()
            .flat_map(|implementation| &implementation.covered_by);
        for slug in covered {
            let message = match rules.iter().find(|(_, other)| other.id == *slug) {
                None => format!("covered-by `{slug}` is not a rule of the catalog"),
                Some((_, other)) if other.is_judgment() => {
                    format!("covered-by `{slug}` is a judgment rule")
                }
                Some(_) => continue,
            };
            found.push(issue(path.as_str(), message));
        }
    }
}
