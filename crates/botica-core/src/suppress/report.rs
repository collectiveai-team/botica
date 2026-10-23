//! The findings about the comments themselves: `invalid-suppression` and `unused-suppression`.

use super::directive::{Malformed, Scope};
use super::message::{
    judgment_on_a_line, judgment_without_reason, malformed_message, project_rule_without_reason,
    project_scope_rule_in_a_comment, unused_message,
};
use super::{Found, Pass, Policy, Token, INVALID, UNUSED};
use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use std::collections::HashSet;
use std::ops::Range;

/// Why a token that silenced nothing is reported: its rule ran on the file, or did not.
enum Unused {
    Unused,
    Disabled,
}

impl Policy {
    fn about(
        &self,
        doc: &Document<'_>,
        rule: &str,
        severity: Severity,
        span: &Range<usize>,
        message: String,
    ) -> Diagnostic {
        let (line, column) = doc.position(span.start);
        let (end_line, end_column) = doc.position(span.end);
        Diagnostic {
            path: doc.path.to_owned(),
            rule: rule.to_owned(),
            severity,
            start: span.start,
            end: span.end,
            line,
            column,
            end_line,
            end_column,
            file_level: false,
            measure: None,
            message,
        }
    }

    /// The findings of `invalid-suppression` about the directives of a file: the unknown rules,
    /// the malformed directives, then the judgment and project rules suppressed the wrong way.
    pub(super) fn invalid_findings(
        &self,
        doc: &Document<'_>,
        severity: Severity,
        directives: &[Found],
        malformed: &[(Range<usize>, Malformed)],
    ) -> Vec<Diagnostic> {
        let unknown = directives
            .iter()
            .flat_map(|directive| &directive.tokens)
            .filter(|token| token.slug.is_none() && !token.external)
            .map(|token| {
                let message = format!("Unknown rule `{}` in a suppression", token.name);
                self.about(doc, INVALID, severity, &token.span, message)
            });
        let malformed = malformed.iter().map(|(span, reason)| {
            self.about(doc, INVALID, severity, span, malformed_message(reason))
        });
        let misplaced = directives
            .iter()
            .flat_map(|directive| self.kind_problems(directive))
            .map(|(token, message)| self.about(doc, INVALID, severity, &token.span, message));
        unknown.chain(malformed).chain(misplaced).collect()
    }

    /// The finding of `unused-suppression` about `directive`, when this pass reports a rule it
    /// names and did not silence. `file_level` holds the rules left with a finding about the whole
    /// file.
    pub(super) fn unused_finding(
        &self,
        doc: &Document<'_>,
        severity: Severity,
        directive: &Found,
        pass: &Pass<'_>,
        file_level: &HashSet<String>,
    ) -> Option<Diagnostic> {
        let mut unused = Vec::new();
        let mut disabled = Vec::new();
        let mut file_hint = Vec::new();
        for token in &directive.tokens {
            let Some(slug) = token.slug.as_deref().filter(|_| !token.used) else {
                continue;
            };
            match self.unused_kind(slug, pass) {
                None => {}
                Some(Unused::Disabled) => disabled.push(token.name.as_str()),
                Some(Unused::Unused) => {
                    unused.push(token.name.as_str());
                    if directive.scope == Scope::Line && file_level.contains(slug) {
                        file_hint.push(token.name.as_str());
                    }
                }
            }
        }
        let message = unused_message(&unused, &disabled, &file_hint)?;
        Some(self.about(doc, UNUSED, severity, &directive.span, message))
    }

    /// Whether a token that silenced nothing is reported in this pass, and how.
    fn unused_kind(&self, slug: &str, pass: &Pass<'_>) -> Option<Unused> {
        if slug == UNUSED
            || slug == INVALID
            || self.judgment.contains(slug)
            || self.project.contains(slug)
        {
            return None;
        }
        let on_a_line = self.project_lines.contains(slug);
        if on_a_line || matches!(pass, Pass::Project) {
            return (on_a_line && matches!(pass, Pass::Project)).then_some(Unused::Unused);
        }
        let extension = self.extension.get(slug).copied();
        match (pass, extension) {
            (Pass::Silence | Pass::Project, _)
            | (Pass::Walk(_), Some(true))
            | (Pass::Extensions(_), None | Some(false)) => None,
            (Pass::Walk(_), Some(false)) => Some(Unused::Disabled),
            (Pass::Walk(is_active), None) => Some(if is_active(slug) {
                Unused::Unused
            } else {
                Unused::Disabled
            }),
            (Pass::Extensions(ran), Some(true)) => ran(slug).map(|read| {
                if read {
                    Unused::Unused
                } else {
                    Unused::Disabled
                }
            }),
        }
    }

    /// What is wrong with how a directive suppresses the judgment rules and the rules of project
    /// scope it names: a rule of project scope whose findings sit on a line needs a reason.
    fn kind_problems<'a>(&self, directive: &'a Found) -> Vec<(&'a Token, String)> {
        directive
            .tokens
            .iter()
            .filter_map(|token| {
                let slug = token.slug.as_deref()?;
                if self.project_lines.contains(slug) {
                    return (!directive.reason)
                        .then(|| (token, project_rule_without_reason(&token.name)));
                }
                if self.project.contains(slug) {
                    return Some((token, project_scope_rule_in_a_comment(&token.name, slug)));
                }
                if !self.judgment.contains(slug) {
                    return None;
                }
                if directive.scope == Scope::Line {
                    Some((token, judgment_on_a_line(&token.name, slug)))
                } else if !directive.reason {
                    Some((token, judgment_without_reason(&token.name)))
                } else {
                    None
                }
            })
            .collect()
    }
}
