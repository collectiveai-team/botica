//! The variables that a varlock schema declares: the `KEY=value` lines, with the `# @…` decorator
//! comments right above each one.

/// A variable of the schema and where its key is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variable {
    pub name: String,
    /// The byte range of the key.
    pub start: usize,
    pub end: usize,
    /// It carries `@auditIgnore`: another program reads it.
    pub audit_ignore: bool,
}

const AUDIT_IGNORE: &str = "@auditIgnore";

/// The variables of `text`, in order. The decorators of the comment lines right above a variable
/// belong to it; a blank line or the `# ---` line that ends the header breaks that.
pub fn variables(text: &str) -> Vec<Variable> {
    let mut found = Vec::new();
    let mut ignored_above = false;
    let mut closing: Option<&str> = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let line = line.trim_end_matches(['\n', '\r']);
        if let Some(quote) = closing {
            if line.contains(quote) {
                closing = None;
            }
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            ignored_above = false;
        } else if let Some(comment) = trimmed.strip_prefix('#') {
            if comment.trim_start().starts_with("---") {
                ignored_above = false;
            } else {
                ignored_above |= has_audit_ignore(comment);
            }
        } else if let Some((key_at, key, value)) = item(line) {
            found.push(Variable {
                name: key.to_owned(),
                start: start + key_at,
                end: start + key_at + key.len(),
                audit_ignore: ignored_above || value.split(" #").skip(1).any(has_audit_ignore),
            });
            closing = opens_multiline(value.trim_start());
            ignored_above = false;
        } else {
            ignored_above = false;
        }
    }
    found
}

/// The offset of the key, the key and the value of an item line (`KEY=…`, `export KEY=…`).
fn item(line: &str) -> Option<(usize, &str, &str)> {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    let (skipped, rest) = match rest.strip_prefix("export ") {
        Some(after) => {
            let after_trimmed = after.trim_start();
            (indent + rest.len() - after_trimmed.len(), after_trimmed)
        }
        None => (indent, rest),
    };
    let length = rest
        .char_indices()
        .take_while(|&(at, c)| {
            c == '_' || c.is_ascii_alphabetic() || (at > 0 && c.is_ascii_digit())
        })
        .count();
    let (key, after) = rest.split_at(length);
    let value = after.trim_start().strip_prefix('=')?;
    (!key.is_empty()).then_some((skipped, key, value))
}

/// The quote that ends a value that starts with a triple quote or a backtick and does not end on
/// its own line.
fn opens_multiline(value: &str) -> Option<&'static str> {
    ["\"\"\"", "'''", "```", "`"].into_iter().find(|quote| {
        value
            .strip_prefix(quote)
            .is_some_and(|inside| !inside.contains(quote))
    })
}

fn has_audit_ignore(comment: &str) -> bool {
    comment.match_indices(AUDIT_IGNORE).any(|(at, _)| {
        let after = &comment[at + AUDIT_IGNORE.len()..];
        let ends = after
            .chars()
            .next()
            .is_none_or(|c| !(c.is_ascii_alphanumeric() || c == '_'));
        ends && !after.trim_start().starts_with("=false")
    })
}
