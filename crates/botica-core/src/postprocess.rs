use crate::diagnostic::{Diagnostic, Severity};

/// What the configuration says about one rule at one path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The rule reports here, with this severity.
    Report(Severity),
    /// The rule does not apply to this path.
    Drop,
    /// The configuration does not govern this rule: the diagnostic stays as it is.
    Keep,
}

/// A file reached twice (named twice, or named and also walked) is linted twice; its findings
/// must come out once. Only findings equal in every field count as the same one.
fn drop_repeats(sorted: &mut Vec<Diagnostic>) {
    let mut kept: Vec<Diagnostic> = Vec::with_capacity(sorted.len());
    for diagnostic in sorted.drain(..) {
        let repeated = kept
            .iter()
            .rev()
            .take_while(|known| {
                (&known.path, known.start, &known.rule)
                    == (&diagnostic.path, diagnostic.start, &diagnostic.rule)
            })
            .any(|known| *known == diagnostic);
        if !repeated {
            kept.push(diagnostic);
        }
    }
    *sorted = kept;
}

/// Applies the configuration to the diagnostics and puts them in their final order,
/// `(path, byte start, rule)`.
///
/// `verdict` is asked once per diagnostic, in path order, so a caller may cache per path.
pub fn apply(
    mut diagnostics: Vec<Diagnostic>,
    mut verdict: impl FnMut(&str, &str) -> Verdict,
) -> Vec<Diagnostic> {
    diagnostics.sort_by(|a, b| (&a.path, a.start, &a.rule).cmp(&(&b.path, b.start, &b.rule)));
    drop_repeats(&mut diagnostics);
    diagnostics.retain_mut(
        |diagnostic| match verdict(&diagnostic.path, &diagnostic.rule) {
            Verdict::Report(severity) => {
                diagnostic.severity = severity;
                true
            }
            Verdict::Drop => false,
            Verdict::Keep => true,
        },
    );
    diagnostics
}
