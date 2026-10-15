use botica_catalog::{Catalog, Rule, Status, OWN_RULES_DIR};

/// The folder that holds the page of a rule: its family, or the folder of the rules of botica.
pub fn group_of(rule: &Rule) -> &str {
    rule.family.as_deref().unwrap_or(OWN_RULES_DIR)
}

/// The code comes first, in lower case, so that a site which lists the pages by file name lists
/// the rules of a family in the order of their codes (a code ends in three digits).
pub fn file_name(rule: &Rule) -> String {
    format!("{}-{}.md", rule.code.to_lowercase(), rule.id)
}

/// A status or a severity as the word that `rule.toml` writes.
pub fn word(value: &impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .expect("a status and a severity are words")
}

/// The release that added a rule, `unreleased` while no release has.
pub fn added_in(catalog: &Catalog, rule: &Rule) -> String {
    catalog
        .lifecycle(rule)
        .added
        .unwrap_or_else(|| "unreleased".to_owned())
}

/// The status of a rule; a deprecated or removed one says since which release.
pub fn status_text(catalog: &Catalog, rule: &Rule) -> String {
    let name = capitalized(&word(&rule.status));
    if !matches!(rule.status, Status::Deprecated | Status::Removed) {
        return name;
    }
    match catalog.lifecycle(rule).since {
        Some(version) => format!("{name} (since {version})"),
        None => format!("{name} (unreleased)"),
    }
}

/// A JSON value is also a YAML one, so it quotes and escapes what the front matter holds.
fn json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("a rule field is JSON")
}

/// The page of a rule: its front matter, a header made from `rule.toml`, and the prose of its
/// README, which holds nothing that `rule.toml` already says.
pub fn page(catalog: &Catalog, rule: &Rule) -> String {
    format!(
        "{}\n{}{}",
        front_matter(catalog, rule),
        header(catalog, rule),
        rule.readme
    )
}

fn front_matter(catalog: &Catalog, rule: &Rule) -> String {
    let mut lines = vec![
        format!("title: {}", json(&rule.id)),
        format!("subtitle: {}", json(&rule.code)),
        format!("rule: {}", json(&rule.id)),
        format!("code: {}", json(&rule.code)),
    ];
    if let Some(family) = &rule.family {
        lines.push(format!("family: {}", json(family)));
    }
    if let Some(standard) = &rule.standard {
        lines.push(format!("standard: {}", json(standard)));
    }
    lines.extend([
        format!("description: {}", json(&rule.summary)),
        format!("status: {}", json(&rule.status)),
    ]);
    if let Some(added) = catalog.lifecycle(rule).added {
        lines.push(format!("added: {}", json(&added)));
    }
    lines.extend([
        format!("default-enabled: {}", rule.default_enabled),
        format!("default-severity: {}", json(&rule.default_severity)),
        format!("languages: {}", json(&rule.languages())),
        format!("tags: {}", json(&rule.tags)),
        format!("aliases: {}", json(&rule.aliases)),
        format!(
            "readme: {}",
            json(&format!("{}/{}/README.md", group_of(rule), rule.id))
        ),
    ]);
    format!("---\n{}\n---\n", lines.join("\n"))
}

fn header(catalog: &Catalog, rule: &Rule) -> String {
    let mut paragraphs = vec![
        format!("# {} ({})", rule.id, rule.code),
        format!(
            "{} · {} · {}",
            added_text(catalog, rule),
            status_text(catalog, rule),
            default_selection(rule)
        ),
        identity(rule),
    ];
    if !rule.aliases.is_empty() {
        paragraphs.push(format!("Formerly known as: {}", code_list(&rule.aliases)));
    }
    if let Some(old) = &rule.replaces {
        paragraphs.push(format!("Replaces: {}", mention(catalog, old, rule)));
    }
    paragraphs.extend(status_notes(catalog, rule));
    format!("{}\n\n", paragraphs.join("\n\n"))
}

fn added_text(catalog: &Catalog, rule: &Rule) -> String {
    match catalog.lifecycle(rule).added {
        Some(version) => format!("Added in {version}"),
        None => "Not released yet".to_owned(),
    }
}

fn default_selection(rule: &Rule) -> String {
    let selection = if rule.default_enabled {
        "Enabled by default"
    } else {
        "Off by default"
    };
    format!("{selection} ({})", word(&rule.default_severity))
}

fn identity(rule: &Rule) -> String {
    let mut parts = vec![match &rule.family {
        Some(family) => format!("Family: {family}"),
        None => "Rule of botica itself".to_owned(),
    }];
    if let Some(standard) = &rule.standard {
        parts.push(format!("Standard: {standard}"));
    }
    let languages: Vec<&str> = rule
        .languages()
        .into_iter()
        .map(|language| if language == "*" { "all" } else { language })
        .collect();
    parts.push(format!("Languages: {}", languages.join(", ")));
    parts.join(" · ")
}

fn status_notes(catalog: &Catalog, rule: &Rule) -> Vec<String> {
    let mut notes = Vec::new();
    match rule.status {
        Status::Stable => {}
        Status::Preview => notes.push(
            "This rule is unstable and in preview. It runs only with `preview = true`, and only \
             when it is selected."
                .to_owned(),
        ),
        Status::Deprecated => notes.push(
            "**Warning:** this rule is deprecated and will be removed in a future release."
                .to_owned(),
        ),
        Status::Removed => notes.push(
            "**Warning:** this rule has been removed and its documentation is only available for \
             historical reasons."
                .to_owned(),
        ),
    }
    if matches!(rule.status, Status::Deprecated | Status::Removed) {
        notes.extend(replacement_note(catalog, rule));
    }
    notes
}

fn replacement_note(catalog: &Catalog, rule: &Rule) -> Option<String> {
    let replacements: Vec<String> = catalog
        .rules()
        .iter()
        .filter(|other| {
            other
                .replaces
                .as_deref()
                .and_then(|old| catalog.resolve(old))
                .is_some_and(|old| old.id == rule.id)
        })
        .map(|other| link(other, rule))
        .collect();
    (!replacements.is_empty()).then(|| format!("Use {} instead.", replacements.join(", ")))
}

/// A rule named in the page of another: a link when the catalog has it, its name otherwise.
fn mention(catalog: &Catalog, name: &str, from: &Rule) -> String {
    catalog
        .resolve(name)
        .map_or_else(|| format!("`{name}`"), |found| link(found, from))
}

fn link(target: &Rule, from: &Rule) -> String {
    let prefix = if group_of(target) == group_of(from) {
        String::new()
    } else {
        format!("../{}/", group_of(target))
    };
    format!("[`{}`]({prefix}{})", target.id, file_name(target))
}

fn code_list(names: &[String]) -> String {
    names
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use botica_catalog::SourceFile;

    struct Spec<'a> {
        family: &'a str,
        code: &'a str,
        id: &'a str,
        extra: &'a str,
    }

    fn file(path: &str, text: &str) -> SourceFile {
        SourceFile {
            path: path.to_owned(),
            text: text.to_owned(),
        }
    }

    fn catalog(specs: &[Spec<'_>]) -> Catalog {
        catalog_with_history(specs, "")
    }

    fn catalog_with_history(specs: &[Spec<'_>], history: &str) -> Catalog {
        let mut files = vec![
            file("history.toml", history),
            file(
                "logging/family.toml",
                "code = \"LOG\"\ndescription = \"d\"\n",
            ),
            file(
                "process/family.toml",
                "code = \"PROC\"\ndescription = \"d\"\n",
            ),
        ];
        for spec in specs {
            let rule = format!(
                "id = \"{}\"\ncode = \"{}\"\nsummary = \"s\"\nscope = \"file\"\n\
                 needs = \"syntax\"\nrun-on = [\"cli\"]\ndefault-enabled = true\n\
                 default-severity = \"error\"\n{}\n[[implementations]]\nkind = \"ast-grep\"\n\
                 languages = [\"python\"]\nfile = \"python.yml\"\n",
                spec.id, spec.code, spec.extra
            );
            let folder = format!("{}/{}", spec.family, spec.id);
            files.push(file(&format!("{folder}/rule.toml"), &rule));
            files.push(file(&format!("{folder}/README.md"), "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"));
            files.push(file(
                &format!("{folder}/python.yml"),
                &format!("id: {}\n", spec.id),
            ));
        }
        Catalog::from_files(&files).expect("the catalog loads")
    }

    fn page_of(catalog: &Catalog, id: &str) -> String {
        page(catalog, catalog.resolve(id).expect("a rule of the catalog"))
    }

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    #[test]
    fn the_front_matter_names_the_page_by_the_slug_and_subtitles_it_with_the_code() {
        let catalog = catalog(&[Spec {
            family: "logging",
            code: "BTLOG001",
            id: "log-no-print",
            extra: "status = \"stable\"\n",
        }]);

        let page = page_of(&catalog, "log-no-print");

        let front_matter = lines(page.split("\n---\n").next().expect("a front matter"));
        assert!(front_matter.contains(&"title: \"log-no-print\""), "{page}");
        assert!(front_matter.contains(&"subtitle: \"BTLOG001\""), "{page}");
    }

    #[test]
    fn a_deprecated_rule_warns_and_points_to_the_rule_that_replaces_it() {
        let catalog = catalog_with_history(
            &[
                Spec {
                    family: "logging",
                    code: "BTLOG001",
                    id: "old",
                    extra: "status = \"deprecated\"\n",
                },
                Spec {
                    family: "logging",
                    code: "BTLOG002",
                    id: "new",
                    extra: "status = \"stable\"\nreplaces = \"old\"\n",
                },
            ],
            &events(&[
                ("0.1.0", "old", "stable"),
                ("0.2.0", "old", "deprecated"),
                ("0.2.0", "new", "stable"),
            ]),
        );

        let old = page_of(&catalog, "old");
        assert!(
            lines(&old).contains(
                &"Added in 0.1.0 · Deprecated (since 0.2.0) · Enabled by default (error)"
            ),
            "{old}"
        );
        assert!(
            lines(&old).contains(
                &"**Warning:** this rule is deprecated and will be removed in a future release."
            ),
            "{old}"
        );
        assert!(
            lines(&old).contains(&"Use [`new`](btlog002-new.md) instead."),
            "{old}"
        );

        let new = page_of(&catalog, "new");
        assert!(
            lines(&new).contains(&"Replaces: [`old`](btlog001-old.md)"),
            "{new}"
        );
        assert!(!new.contains("Warning"), "{new}");
    }

    #[test]
    fn a_removed_rule_says_it_is_kept_for_history_and_links_across_families() {
        let catalog = catalog_with_history(
            &[
                Spec {
                    family: "logging",
                    code: "BTLOG001",
                    id: "old",
                    extra: "status = \"removed\"\n",
                },
                Spec {
                    family: "process",
                    code: "BTPROC001",
                    id: "new",
                    extra: "status = \"stable\"\nreplaces = \"old\"\n",
                },
            ],
            &events(&[("0.1.0", "old", "stable"), ("0.4.0", "old", "removed")]),
        );

        let old = page_of(&catalog, "old");
        assert!(
            lines(&old)
                .contains(&"Added in 0.1.0 · Removed (since 0.4.0) · Enabled by default (error)"),
            "{old}"
        );
        assert!(
            lines(&old).contains(
                &"**Warning:** this rule has been removed and its documentation is only available for historical reasons."
            ),
            "{old}"
        );
        assert!(
            lines(&old).contains(&"Use [`new`](../process/btproc001-new.md) instead."),
            "{old}"
        );
    }

    #[test]
    fn a_status_that_no_release_has_recorded_yet_says_so() {
        let catalog = catalog_with_history(
            &[Spec {
                family: "logging",
                code: "BTLOG001",
                id: "old",
                extra: "status = \"deprecated\"\n",
            }],
            &events(&[("0.1.0", "old", "stable")]),
        );

        let old = page_of(&catalog, "old");
        assert!(
            lines(&old)
                .contains(&"Added in 0.1.0 · Deprecated (unreleased) · Enabled by default (error)"),
            "{old}"
        );
    }

    #[test]
    fn a_rule_no_release_has_added_is_not_released_yet() {
        let catalog = catalog(&[Spec {
            family: "logging",
            code: "BTLOG001",
            id: "fresh",
            extra: "status = \"preview\"\n",
        }]);

        let fresh = page_of(&catalog, "fresh");
        assert!(
            lines(&fresh).contains(&"Not released yet · Preview · Enabled by default (error)"),
            "{fresh}"
        );
        assert!(!fresh.contains("added:"), "{fresh}");
    }

    fn events(list: &[(&str, &str, &str)]) -> String {
        list.iter()
            .map(|(version, rule, status)| {
                format!(
                    "[[event]]\nversion = \"{version}\"\nrule = \"{rule}\"\nstatus = \"{status}\"\n"
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn a_stable_rule_carries_no_note_and_a_rule_that_replaces_nothing_in_the_catalog_names_it() {
        let catalog = catalog(&[Spec {
            family: "logging",
            code: "BTLOG001",
            id: "only",
            extra: "status = \"stable\"\nreplaces = \"ghost\"\naliases = [\"first\", \"second\"]\n",
        }]);

        let only = page_of(&catalog, "only");
        assert!(lines(&only).contains(&"Replaces: `ghost`"), "{only}");
        assert!(
            lines(&only).contains(&"Formerly known as: `first`, `second`"),
            "{only}"
        );
        assert!(
            !only.contains("Warning") && !only.contains("unstable"),
            "{only}"
        );
    }

    #[test]
    fn a_summary_with_quotes_stays_one_value_of_the_front_matter() {
        let catalog = catalog(&[Spec {
            family: "logging",
            code: "BTLOG001",
            id: "quoted",
            extra: "status = \"stable\"\n",
        }]);
        let mut rule = catalog.resolve("quoted").expect("the rule").clone();
        rule.summary = "named \"{Name}Request\": yes".to_owned();

        let text = page(&catalog, &rule);

        assert!(
            text.contains("description: \"named \\\"{Name}Request\\\": yes\"\n"),
            "{text}"
        );
    }
}
