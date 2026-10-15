use serde::Serialize;

use crate::model::Rule;

/// A section of the README of a rule: the text of a `## ` heading and what follows it up to the
/// next one, without the blank lines around it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub title: String,
    pub text: String,
}

impl Rule {
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
