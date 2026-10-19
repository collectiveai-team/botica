//! In-source suppressions (spec §8): `# botica: ignore[...]`, `# botica: ignore-file[...]` and
//! `# noqa: BT...`, plus the two rules about them, `unused-suppression` and `invalid-suppression`.

mod directive;
mod message;
mod scan;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use aho_corasick::AhoCorasick;
use directive::{Channels, Directive, Malformed, Scope};
use message::{judgment_on_a_line, judgment_without_reason, malformed_message, unused_message};
use scan::{allows_noqa, scan, Syntax};
use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::OnceLock;

pub const UNUSED: &str = "unused-suppression";
pub const INVALID: &str = "invalid-suppression";

type External = Box<dyn Fn(&str) -> bool + Send + Sync>;

/// What a suppression may name and how the two rules about suppressions report.
pub struct Policy {
    /// Every spelling of a rule (slug, code, alias) -> its slug.
    spellings: HashMap<String, String>,
    /// Rules run by the ast-grep set: they are active in a file when the set covers it.
    astgrep: HashSet<String>,
    /// Judgment rules: suppressed by file only, with a reason, and never unused.
    judgment: HashSet<String>,
    /// Rules that an extension runs after the walk, each with whether it is active. The walk
    /// reports a comment that names an inactive one; [`Policy::report_extensions`] reports the
    /// others once the extensions have run.
    extension: HashMap<String, bool>,
    external: Option<External>,
    /// What the codes in a `noqa` comment start with when they are botica's.
    code_prefixes: Vec<String>,
    unused: Option<Severity>,
    invalid: Option<Severity>,
    /// One pass over a file tells whether it holds any marker at all.
    markers: OnceLock<AhoCorasick>,
}

impl Default for Policy {
    fn default() -> Self {
        Self::new()
    }
}

impl Policy {
    /// The two rules about suppressions are always known; they report only after
    /// `report_unused` and `report_invalid`.
    pub fn new() -> Self {
        Self {
            spellings: HashMap::new(),
            astgrep: HashSet::new(),
            judgment: HashSet::new(),
            extension: HashMap::new(),
            external: None,
            code_prefixes: vec!["BT".to_owned()],
            unused: None,
            invalid: None,
            markers: OnceLock::new(),
        }
        .with_rule(UNUSED, &["BT002"])
        .with_rule(INVALID, &["BT003"])
    }

    /// A rule a suppression may name, by its slug and by any other spelling.
    pub fn with_rule(mut self, slug: &str, spellings: &[&str]) -> Self {
        self.spellings.insert(slug.to_owned(), slug.to_owned());
        for spelling in spellings {
            self.spellings
                .insert((*spelling).to_owned(), slug.to_owned());
        }
        self
    }

    /// A judgment rule a suppression may name: a judge evaluates it, and botica never runs it.
    pub fn with_judgment_rule(mut self, slug: &str, spellings: &[&str]) -> Self {
        self.judgment.insert(slug.to_owned());
        self.with_rule(slug, spellings)
    }

    /// A rule that an extension runs after the walk. The engine does not see its findings, so a
    /// comment that names an active one is reported by [`Policy::report_extensions`], after the
    /// extensions have run.
    pub fn with_extension_rule(mut self, slug: &str, spellings: &[&str], active: bool) -> Self {
        self.extension.insert(slug.to_owned(), active);
        self.with_rule(slug, spellings)
    }

    /// Rules of the ast-grep set, known by slug only.
    pub fn with_astgrep_rules<'a>(mut self, slugs: impl IntoIterator<Item = &'a str>) -> Self {
        for slug in slugs {
            self.spellings.insert(slug.to_owned(), slug.to_owned());
            self.astgrep.insert(slug.to_owned());
        }
        self
    }

    /// The code of a catalog of the project: a `noqa` code that starts with it is a code of botica,
    /// and one that names no rule is reported as unknown.
    pub fn with_code_prefix(mut self, prefix: &str) -> Self {
        self.code_prefixes.push(prefix.to_owned());
        self
    }

    /// IDs that suppressions may name even though no rule has them.
    pub fn with_external(
        mut self,
        is_external: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.external = Some(Box::new(is_external));
        self
    }

    pub fn report_unused(mut self, severity: Severity) -> Self {
        self.unused = Some(severity);
        self
    }

    pub fn report_invalid(mut self, severity: Severity) -> Self {
        self.invalid = Some(severity);
        self
    }

    fn has_marker(&self, text: &str) -> bool {
        self.markers
            .get_or_init(|| {
                AhoCorasick::new(["botica:", "noqa", "NOQA"]).expect("literal needles compile")
            })
            .is_match(text)
    }

    /// Whether this policy would report about the comments of the file at `path`.
    pub(crate) fn reads(&self, path: &str) -> bool {
        (self.unused.is_some() || self.invalid.is_some()) && Syntax::of(path).is_some()
    }

    /// Whether a comment of the file at `path` could silence a finding there, so the file has to
    /// be read even when the rules look only at its name.
    pub(crate) fn can_silence(&self, path: &str) -> bool {
        Syntax::of(path).is_some()
    }

    pub(crate) fn is_astgrep(&self, slug: &str) -> bool {
        self.astgrep.contains(slug)
    }
}

/// An id inside a directive, resolved.
struct Token {
    name: String,
    span: Range<usize>,
    /// The slug of the rule it names; `None` for an unknown id.
    slug: Option<String>,
    external: bool,
    used: bool,
}

struct Found {
    scope: Scope,
    line: usize,
    span: Range<usize>,
    tokens: Vec<Token>,
    reason: bool,
}

impl Policy {
    /// The line a finding counts on: where it starts, or the last line of the multi-line string it
    /// starts in. `None` for a finding about the whole file.
    fn effective_line(
        diagnostic: &Diagnostic,
        strings: &[Range<usize>],
        doc: &Document<'_>,
    ) -> Option<usize> {
        if diagnostic.file_level {
            return None;
        }
        let inside = strings
            .iter()
            .find(|string| string.contains(&diagnostic.start));
        Some(match inside {
            Some(string) => doc.position(string.end.saturating_sub(1)).0,
            None => diagnostic.line,
        })
    }

    /// Marks the tokens that silence `diagnostic` and says whether any did.
    fn silenced(found: &mut [Found], diagnostic: &Diagnostic, line: Option<usize>) -> bool {
        let mut silenced = false;
        for directive in found.iter_mut() {
            let reaches = match directive.scope {
                Scope::File => true,
                Scope::Line => line == Some(directive.line),
            };
            if !reaches {
                continue;
            }
            for token in &mut directive.tokens {
                if token.slug.as_deref() == Some(diagnostic.rule.as_str()) {
                    token.used = true;
                    silenced = true;
                }
            }
        }
        silenced
    }

    fn resolve(
        &self,
        directives: Vec<Directive>,
        doc: &Document<'_>,
    ) -> (Vec<Found>, Vec<(Range<usize>, Malformed)>) {
        let mut found = Vec::new();
        let mut malformed = Vec::new();
        for directive in directives {
            match directive {
                Directive::Malformed { span, reason } => malformed.push((span, reason)),
                Directive::Suppress {
                    scope,
                    channel: _,
                    ids,
                    span,
                    reason,
                } => {
                    let tokens = ids
                        .into_iter()
                        .map(|id| {
                            let slug = self.spellings.get(&id.name).cloned();
                            let external = slug.is_none()
                                && self.external.as_ref().is_some_and(|f| f(&id.name));
                            Token {
                                name: id.name,
                                span: id.span,
                                slug,
                                external,
                                used: false,
                            }
                        })
                        .collect();
                    let line = doc.position(span.start).0;
                    found.push(Found {
                        scope,
                        line,
                        span,
                        tokens,
                        reason,
                    });
                }
            }
        }
        (found, malformed)
    }

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
            message,
        }
    }

    /// Drops from `found` what the comments of the file at `path` silence: the findings of a
    /// judge, which botica reads apart from the rules it runs. It adds no finding about the
    /// comments.
    pub fn silence(&self, path: &str, text: &str, found: &mut Vec<Diagnostic>) {
        self.filter(&Document::new(path, text), &Pass::Silence, found);
    }

    /// Drops from `found`, the findings of the extension rules in the file at `path`, what its
    /// comments silence, and adds a finding about each comment that names an active extension
    /// rule and silences nothing. `ran` says whether the extension rule read the file: `None`
    /// when its process failed, and then its comments are not reported.
    pub fn report_extensions(
        &self,
        path: &str,
        text: &str,
        ran: &dyn Fn(&str) -> Option<bool>,
        found: &mut Vec<Diagnostic>,
    ) {
        self.filter(&Document::new(path, text), &Pass::Extensions(ran), found);
    }

    /// Drops what the comments of the file silence and adds the findings about the comments.
    pub(crate) fn apply(
        &self,
        doc: &Document<'_>,
        is_active: &dyn Fn(&str) -> bool,
        found: &mut Vec<Diagnostic>,
    ) {
        self.filter(doc, &Pass::Walk(is_active), found);
    }

    /// Whether a token that silenced nothing is reported in this pass, and how.
    fn unused_kind(&self, slug: &str, pass: &Pass<'_>) -> Option<Unused> {
        if slug == UNUSED || slug == INVALID || self.judgment.contains(slug) {
            return None;
        }
        let extension = self.extension.get(slug).copied();
        match (pass, extension) {
            (Pass::Silence, _)
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

    fn filter(&self, doc: &Document<'_>, pass: &Pass<'_>, found: &mut Vec<Diagnostic>) {
        let Some(syntax) = Syntax::of(doc.path) else {
            return;
        };
        let channels = Channels {
            noqa: allows_noqa(doc.path),
            prefixes: &self.code_prefixes,
        };
        let Some(text) = doc.text() else {
            return;
        };
        if !self.has_marker(text) {
            return;
        }
        let scanned = scan(text, syntax);
        let mut directives = Vec::new();
        for comment in &scanned.comments {
            directive::parse(
                &text[comment.clone()],
                comment.start,
                channels,
                &mut directives,
            );
        }
        if directives.is_empty() {
            return;
        }
        let (mut directives, malformed) = self.resolve(directives, doc);

        let mut kept = Vec::with_capacity(found.len());
        let mut file_level: HashSet<String> = HashSet::new();
        for diagnostic in found.drain(..) {
            let line = Self::effective_line(&diagnostic, &scanned.strings, doc);
            if Self::silenced(&mut directives, &diagnostic, line) {
                continue;
            }
            if diagnostic.file_level {
                file_level.insert(diagnostic.rule.clone());
            }
            kept.push(diagnostic);
        }

        let mut about = Vec::new();
        let walk = matches!(pass, Pass::Walk(_));
        if let Some(severity) = self.invalid.filter(|_| walk) {
            for directive in &directives {
                for token in directive
                    .tokens
                    .iter()
                    .filter(|t| t.slug.is_none() && !t.external)
                {
                    about.push(self.about(
                        doc,
                        INVALID,
                        severity,
                        &token.span,
                        format!("Unknown rule `{}` in a suppression", token.name),
                    ));
                }
            }
            for (span, reason) in &malformed {
                about.push(self.about(doc, INVALID, severity, span, malformed_message(reason)));
            }
            for directive in &directives {
                for (token, message) in self.judgment_problems(directive) {
                    about.push(self.about(doc, INVALID, severity, &token.span, message));
                }
            }
        }
        if let Some(severity) = self.unused.filter(|_| !matches!(pass, Pass::Silence)) {
            for directive in &directives {
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
                if let Some(message) = unused_message(&unused, &disabled, &file_hint) {
                    about.push(self.about(doc, UNUSED, severity, &directive.span, message));
                }
            }
        }
        for diagnostic in about {
            let line = Self::effective_line(&diagnostic, &scanned.strings, doc);
            if !Self::silenced(&mut directives, &diagnostic, line) {
                kept.push(diagnostic);
            }
        }
        *found = kept;
    }

    /// What is wrong with how a directive suppresses the judgment rules it names.
    fn judgment_problems<'a>(&self, directive: &'a Found) -> Vec<(&'a Token, String)> {
        directive
            .tokens
            .iter()
            .filter_map(|token| {
                let slug = token.slug.as_deref()?;
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

/// A pass over the comments of a file.
enum Pass<'a> {
    /// It drops what the comments silence and reports nothing.
    Silence,
    /// The walk of the engine: it reports the comments; the function says whether the engine runs
    /// a rule on the file.
    Walk(&'a dyn Fn(&str) -> bool),
    /// After the extensions: it reports the comments that name an active extension rule; the
    /// function says whether the rule read the file, `None` when its process failed.
    Extensions(&'a dyn Fn(&str) -> Option<bool>),
}

/// Why a token that silenced nothing is reported: its rule ran on the file, or did not.
enum Unused {
    Unused,
    Disabled,
}
