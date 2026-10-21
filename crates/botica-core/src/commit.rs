//! The rules of scope `commit`: they read a commit message, never a file (ADR 0010).

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;

pub const CONVENTIONAL_COMMITS: &str = "conventional-commits";
pub const NO_AI_COAUTHORSHIP: &str = "no-ai-coauthorship";

/// A commit message, and where a finding in it is: `commit <short sha>`, the file of the message,
/// or `pull request title`.
pub struct Message<'a> {
    pub place: &'a str,
    pub text: &'a str,
}

pub trait MessageRule: Send + Sync {
    fn id(&self) -> &'static str;

    fn check(&self, message: &Message<'_>, out: &mut Vec<Diagnostic>);
}

/// The message as git commits it from the file a `commit-msg` hook gets: the lines that start
/// with `#` blanked, and nothing from the scissors line of `git commit --verbose` on. The lines
/// keep their numbers.
pub fn without_comments(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        if line.starts_with("# ------------------------ >8 ------------------------") {
            break;
        }
        if line.starts_with('#') {
            kept.push('\n');
        } else {
            kept.push_str(line);
        }
    }
    kept
}

/// A finding over `range` of the message.
fn finding(
    message: &Message<'_>,
    rule: &str,
    range: std::ops::Range<usize>,
    text: String,
) -> Diagnostic {
    let doc = Document::new(message.place, message.text);
    let (line, column) = doc.position(range.start);
    let (end_line, end_column) = doc.position(range.end);
    Diagnostic {
        path: message.place.to_owned(),
        rule: rule.to_owned(),
        severity: Severity::Warning,
        start: range.start,
        end: range.end,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        measure: None,
        message: text,
    }
}

pub struct ConventionalCommits {
    pub types: Vec<String>,
    /// The starts of a subject that is not checked, such as `Merge `.
    pub exempt: Vec<String>,
}

impl ConventionalCommits {
    /// Whether `subject` is `<type>(<scope>)!: <description>`, the scope and the `!` optional.
    fn follows(&self, subject: &str) -> bool {
        let Some((head, description)) = subject.split_once(':') else {
            return false;
        };
        let has_description = description
            .strip_prefix(' ')
            .is_some_and(|rest| !rest.is_empty());
        let head = head.strip_suffix('!').unwrap_or(head);
        let (kind, scope) = match head.split_once('(') {
            Some((kind, rest)) => match rest.strip_suffix(')') {
                Some(scope) => (kind, Some(scope)),
                None => return false,
            },
            None => (head, None),
        };
        let scope_ok = scope.is_none_or(|scope| {
            !scope.is_empty()
                && scope.chars().all(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')
                })
        });
        has_description && scope_ok && self.types.iter().any(|known| known == kind)
    }
}

impl MessageRule for ConventionalCommits {
    fn id(&self) -> &'static str {
        CONVENTIONAL_COMMITS
    }

    fn check(&self, message: &Message<'_>, out: &mut Vec<Diagnostic>) {
        let first = message.text.split('\n').next().unwrap_or_default();
        let subject = first.strip_suffix('\r').unwrap_or(first);
        if self
            .exempt
            .iter()
            .any(|start| subject.starts_with(start.as_str()))
            || self.follows(subject)
        {
            return;
        }
        out.push(finding(
            message,
            CONVENTIONAL_COMMITS,
            0..subject.len(),
            format!(
                "The subject `{subject}` is not `<type>(<scope>): <description>`: start it with \
                 one of {}, an optional lowercase scope in parentheses, `: ` and the description.",
                self.types.join(", ")
            ),
        ));
    }
}

pub struct NoAiCoauthorship {
    /// The names of AI tools, matched in any case.
    pub tools: Vec<String>,
}

/// Whether a line, in lowercase and without its indentation, is a trailer of co-authorship or of
/// attribution: `Co-authored-by:`, `Generated with` or `by`, `AI-generated-by`, `Assisted-by`.
fn is_attribution(line: &str) -> bool {
    if ["co-authored-by:", "ai-generated-by", "assisted-by"]
        .iter()
        .any(|start| line.starts_with(start))
    {
        return true;
    }
    let Some(rest) = line.strip_prefix("generated") else {
        return false;
    };
    let after = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '-');
    after.len() < rest.len() && (after.starts_with("with") || after.starts_with("by"))
}

impl MessageRule for NoAiCoauthorship {
    fn id(&self) -> &'static str {
        NO_AI_COAUTHORSHIP
    }

    fn check(&self, message: &Message<'_>, out: &mut Vec<Diagnostic>) {
        let mut start = 0;
        for line in message.text.split_inclusive('\n') {
            let body = line.trim_end_matches(['\n', '\r']);
            let lowered = body.trim_start().to_lowercase();
            let names_a_tool = self
                .tools
                .iter()
                .any(|tool| lowered.contains(&tool.to_lowercase()));
            if is_attribution(&lowered) && names_a_tool {
                let indent = body.len() - body.trim_start().len();
                out.push(finding(
                    message,
                    NO_AI_COAUTHORSHIP,
                    start + indent..start + body.len(),
                    format!(
                        "`{}` names an AI tool as a co-author or as the author of the change: \
                         remove the line, since the commit names the person who submits it.",
                        body.trim_start()
                    ),
                ));
            }
            start += line.len();
        }
    }
}
