use serde::Serialize;

use crate::issue::{issue, CatalogIssue};
use crate::model::Rule;

/// A section of the README of a rule: the text of a `## ` heading and what follows it up to the
/// next one, without the blank lines around it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub title: String,
    pub text: String,
}

impl Rule {
    /// The text of the Directive, with one space between words and a blank line between
    /// paragraphs.
    pub fn directive(&self) -> String {
        let text = self
            .sections()
            .into_iter()
            .find(|section| section.title == "Directive")
            .map(|section| section.text)
            .unwrap_or_default();
        text.split("\n\n")
            .map(|paragraph| paragraph.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|paragraph| !paragraph.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// The sections of the README, in their order. A `## ` line inside a code block belongs to
    /// the text of its section.
    pub fn sections(&self) -> Vec<Section> {
        let mut sections: Vec<(String, Vec<&str>)> = Vec::new();
        let mut in_code = false;
        for line in self.readme.lines() {
            if line.trim_start().starts_with("```") {
                in_code = !in_code;
            }
            match line.strip_prefix("## ") {
                Some(title) if !in_code => sections.push((title.trim().to_owned(), Vec::new())),
                _ => {
                    if let Some((_, lines)) = sections.last_mut() {
                        lines.push(line);
                    }
                }
            }
        }
        sections
            .into_iter()
            .map(|(title, lines)| Section {
                title,
                text: lines.join("\n").trim_matches('\n').to_owned(),
            })
            .collect()
    }
}

/// The sections of the README of a rule of a family, in their order, and whether each is required.
const FAMILY_TEMPLATE: [(&str, bool); 7] = [
    ("Directive", true),
    ("Why", true),
    ("Exceptions", false),
    ("Do", true),
    ("Don't", true),
    ("Options", false),
    ("Suppression", true),
];

/// The sections of the README of a rule that another tool enforces, in one language at least.
const EXTERNAL_TEMPLATE: [(&str, bool); 9] = [
    ("Directive", true),
    ("Why", true),
    ("Exceptions", false),
    ("Tool", true),
    ("Configuration", true),
    ("Do", true),
    ("Don't", true),
    ("Options", false),
    ("Suppression", true),
];

/// The longest Directive of a judgment rule: it is the `text` of the rule in the rubric of abide.
const MAX_DIRECTIVE: usize = 600;

/// The sections of the README of a judgment rule, all required.
const JUDGMENT_TEMPLATE: [(&str, bool); 8] = [
    ("Directive", true),
    ("Why", true),
    ("Flag", true),
    ("Do not flag", true),
    ("Confirm", true),
    ("Do", true),
    ("Don't", true),
    ("Suppression", true),
];

/// The sections of the README of a rule of botica itself.
const OWN_TEMPLATE: [(&str, bool); 3] = [("Directive", true), ("Why", true), ("Suppression", true)];

fn has_text_before_the_first_section(readme: &str) -> bool {
    readme
        .lines()
        .take_while(|line| !line.starts_with("## "))
        .any(|line| !line.trim().is_empty())
}

/// What is wrong with the sections of the README of a rule: each is one of its template, in the
/// order of the template, and every required one is there.
pub(crate) fn check_readme(path: &str, rule: &Rule) -> Vec<CatalogIssue> {
    let template: &[(&str, bool)] = if rule.family.is_none() {
        &OWN_TEMPLATE
    } else if rule.is_judgment() {
        &JUDGMENT_TEMPLATE
    } else if rule.has_external() {
        &EXTERNAL_TEMPLATE
    } else {
        &FAMILY_TEMPLATE
    };
    let names: Vec<&str> = template.iter().map(|(title, _)| *title).collect();
    let mut found = Vec::new();
    let mut add = |message: String| found.push(issue(path, message));

    if has_text_before_the_first_section(&rule.readme) {
        add(format!("text before `## {}`, the first section", names[0]));
    }
    let sections = rule.sections();
    let mut next = 0;
    let mut previous = "";
    for section in &sections {
        match names.iter().position(|name| *name == section.title) {
            None => add(format!(
                "section `{}` is not in the template: {}",
                section.title,
                names.join(", ")
            )),
            Some(at) if at < next => add(format!(
                "section `{}` comes after `{previous}`, and the template puts it before",
                section.title
            )),
            Some(at) => {
                next = at + 1;
                previous = names[at];
            }
        }
    }
    for (title, required) in template {
        if *required && !sections.iter().any(|section| section.title == *title) {
            add(format!("missing section `{title}`"));
        }
    }
    let length = rule.directive().chars().count();
    if rule.is_judgment() && length > MAX_DIRECTIVE {
        add(format!(
            "the Directive of a judgment rule has at most {MAX_DIRECTIVE} characters, and this one \
             has {length}"
        ));
    }
    found
}
