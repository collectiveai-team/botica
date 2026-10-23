//! In-source suppressions (spec §8): `# botica: ignore[...]`, `# botica: ignore-file[...]` and
//! `# noqa: BT...`, plus the two rules about them, `unused-suppression` and `invalid-suppression`.

mod directive;
mod message;
mod report;
mod scan;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use aho_corasick::AhoCorasick;
use directive::{Channels, Directive, Malformed, Scope};
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
    /// Rules of project scope: the configuration silences them, and a comment that names one is
    /// invalid.
    project: HashSet<String>,
    /// Rules of project scope whose findings sit on a line of a file: a comment with a reason
    /// silences them, and [`Policy::report_project`] reports the ones that silence nothing.
    project_lines: HashSet<String>,
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
            project: HashSet::new(),
            project_lines: HashSet::new(),
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

    /// A rule of project scope a suppression may name: its findings are about the project, and the
    /// configuration silences them.
    pub fn with_project_rule(mut self, slug: &str, spellings: &[&str]) -> Self {
        self.project.insert(slug.to_owned());
        self.with_rule(slug, spellings)
    }

    /// A rule of project scope whose findings sit on a line of a file a suppression may name: a
    /// comment on that line, or an `ignore-file` comment of the file, silences it when it gives
    /// a reason.
    pub fn with_line_project_rule(mut self, slug: &str, spellings: &[&str]) -> Self {
        self.project_lines.insert(slug.to_owned());
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

    /// Drops from `found`, the findings of the rules of project scope in the file at `path` that
    /// a comment may silence, what its comments silence, and adds a finding about each comment
    /// that names one of those rules and silences nothing.
    pub fn report_project(&self, path: &str, text: &str, found: &mut Vec<Diagnostic>) {
        self.filter(&Document::new(path, text), &Pass::Project, found);
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

    fn filter(&self, doc: &Document<'_>, pass: &Pass<'_>, found: &mut Vec<Diagnostic>) {
        let Some(syntax) = Syntax::of(doc.path) else {
            return;
        };
        let Some(text) = doc.text() else {
            return;
        };
        if !self.has_marker(text) {
            return;
        }
        let scanned = scan(text, syntax);
        let directives = self.directives(doc.path, text, &scanned.comments);
        if directives.is_empty() {
            return;
        }
        let (mut directives, malformed) = self.resolve(directives, doc);
        let (mut kept, file_level) = Self::unsilenced(
            std::mem::take(found),
            &mut directives,
            &scanned.strings,
            doc,
        );

        let mut about = Vec::new();
        if let Some(severity) = self.invalid.filter(|_| matches!(pass, Pass::Walk(_))) {
            about.extend(self.invalid_findings(doc, severity, &directives, &malformed));
        }
        if let Some(severity) = self.unused.filter(|_| !matches!(pass, Pass::Silence)) {
            about.extend(directives.iter().filter_map(|directive| {
                self.unused_finding(doc, severity, directive, pass, &file_level)
            }));
        }
        let (about, _) = Self::unsilenced(about, &mut directives, &scanned.strings, doc);
        kept.extend(about);
        *found = kept;
    }

    /// The directives in the comments of the file at `path`.
    fn directives(&self, path: &str, text: &str, comments: &[Range<usize>]) -> Vec<Directive> {
        let channels = Channels {
            noqa: allows_noqa(path),
            prefixes: &self.code_prefixes,
        };
        let mut directives = Vec::new();
        for comment in comments {
            directive::parse(
                &text[comment.clone()],
                comment.start,
                channels,
                &mut directives,
            );
        }
        directives
    }

    /// The findings that no directive silences, marking the tokens that silence the others, and
    /// the rules of the kept findings about a whole file.
    fn unsilenced(
        found: Vec<Diagnostic>,
        directives: &mut [Found],
        strings: &[Range<usize>],
        doc: &Document<'_>,
    ) -> (Vec<Diagnostic>, HashSet<String>) {
        let mut kept = Vec::with_capacity(found.len());
        let mut file_level = HashSet::new();
        for diagnostic in found {
            let line = Self::effective_line(&diagnostic, strings, doc);
            if Self::silenced(directives, &diagnostic, line) {
                continue;
            }
            if diagnostic.file_level {
                file_level.insert(diagnostic.rule.clone());
            }
            kept.push(diagnostic);
        }
        (kept, file_level)
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
    /// After the rules of project scope: it reports the comments that name one whose findings
    /// sit on a line.
    Project,
}
