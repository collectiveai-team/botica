mod buffer;
mod locate;
mod scan;

pub use buffer::Buffer;

use crate::chains;
use crate::language::language_of;
use crate::rule::AstGrepRule;
use crate::rule_timing::{self, RuleTime};
use ast_grep_core::Matcher;
use ast_grep_language::SupportLang;
use memchr::memmem::Finder;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Rules of one language, indexed by the node kinds they can match, so a file is walked once
/// no matter how many rules apply (the same idea as ast-grep's own `CombinedScan`).
struct LanguageGroup {
    language: SupportLang,
    rules: Vec<usize>,
    by_kind: Vec<Vec<usize>>,
    chains: Option<chains::LangChains>,
}

impl LanguageGroup {
    fn new(language: SupportLang) -> Self {
        Self {
            language,
            rules: Vec::new(),
            by_kind: Vec::new(),
            chains: None,
        }
    }

    fn add(&mut self, index: usize, rule: &AstGrepRule) {
        self.rules.push(index);
        let Some(kinds) = rule.config.matcher.potential_kinds() else {
            return;
        };
        for kind in kinds.iter() {
            if self.by_kind.len() <= kind {
                self.by_kind.resize(kind + 1, Vec::new());
            }
            self.by_kind[kind].push(index);
        }
    }

    /// Sets the prefilter of each rule of the group, declared or derived, and builds the token
    /// chains the derived ones look for.
    fn derive_requirements(&mut self, rules: &mut [AstGrepRule]) {
        let mut table = chains::LangChains::new(self.language);
        for &index in &self.rules {
            let rule = &mut rules[index];
            rule.requirement = match &rule.declared {
                Some(declared) => declared.clone(),
                None => table.requirement(&rule.rule_json),
            };
        }
        table.finish();
        self.chains = (!table.is_empty()).then_some(table);
    }
}

fn group_by_language(rules: &[AstGrepRule]) -> Vec<LanguageGroup> {
    let mut groups: Vec<LanguageGroup> = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        let language = rule.language();
        let position = match groups.iter().position(|g| g.language == language) {
            Some(position) => position,
            None => {
                groups.push(LanguageGroup::new(language));
                groups.len() - 1
            }
        };
        groups[position].add(index, rule);
    }
    groups
}

type Literals = HashMap<String, Finder<'static>>;
type Regexes = HashMap<String, regex::bytes::Regex>;

/// The searchers of every literal and regex the prefilters of `rules` look for.
fn prefilter_searchers(rules: &[AstGrepRule]) -> (Literals, Regexes) {
    let mut literals = Literals::new();
    let mut regexes = Regexes::new();
    for rule in rules {
        let mut needed = Vec::new();
        rule.requirement.literals(&mut needed);
        for literal in needed {
            literals
                .entry(literal.to_owned())
                .or_insert_with(|| Finder::new(literal.as_bytes()).into_owned());
        }
        let mut patterns = Vec::new();
        rule.requirement.regexes(&mut patterns);
        for pattern in patterns {
            regexes.entry(pattern.to_owned()).or_insert_with(|| {
                regex::bytes::Regex::new(pattern).expect("valid prefilter regex")
            });
        }
    }
    (literals, regexes)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PrefilterStats {
    pub parsed: usize,
    pub skipped: usize,
    /// Files parsed by regions, files whose partial tree had errors (re-parsed whole), and the
    /// bytes handed to the parser against the bytes of the parsed files.
    pub region_files: usize,
    pub region_fallbacks: usize,
    /// Files the lexer flagged as likely syntax errors, parsed whole to match ast-grep.
    pub region_suspects: usize,
    /// Files matched skipping the subtrees without candidates.
    pub anchored_files: usize,
    pub parsed_bytes: usize,
    pub file_bytes: usize,
}

/// Which optimisations a set uses. They change how fast the hits are found, never which hits:
/// switching them off is how the conformance runner checks exactly that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tuning {
    /// Parse only the files whose text can hold a match, found by each rule's derived prefilter.
    pub prefilter: bool,
    /// Parse only the top-level statements holding a candidate (Python, JS/TS).
    pub regions: bool,
    /// Skip, while matching, the subtrees that hold no candidate position.
    pub skip_subtrees: bool,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            prefilter: true,
            regions: true,
            skip_subtrees: true,
        }
    }
}

pub struct AstGrepSet {
    rules: Vec<AstGrepRule>,
    groups: Vec<LanguageGroup>,
    literals: Literals,
    regexes: Regexes,
    prefilter: bool,
    regions: bool,
    skip_subtrees: bool,
    parsed: AtomicUsize,
    skipped: AtomicUsize,
    region_files: AtomicUsize,
    region_fallbacks: AtomicUsize,
    region_suspects: AtomicUsize,
    anchored_files: AtomicUsize,
    parsed_bytes: AtomicUsize,
    file_bytes: AtomicUsize,
    rule_timings: Option<rule_timing::RuleTimings>,
}

impl Default for AstGrepSet {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl AstGrepSet {
    pub fn new(rules: Vec<AstGrepRule>) -> Self {
        Self::with_tuning(rules, Tuning::default())
    }

    /// Groups the rules by language and derives each rule's prefilter from the rule itself
    /// (token chains of its patterns, literals of its regexes); declared prefilters win.
    /// Regions and subtree skipping both read the prefilter's candidate positions, so they go
    /// off with it.
    pub fn with_tuning(mut rules: Vec<AstGrepRule>, tuning: Tuning) -> Self {
        let mut groups = group_by_language(&rules);
        for group in &mut groups {
            group.derive_requirements(&mut rules);
        }
        let (literals, regexes) = prefilter_searchers(&rules);
        Self {
            rules,
            groups,
            literals,
            regexes,
            prefilter: tuning.prefilter,
            regions: tuning.regions && tuning.prefilter,
            skip_subtrees: tuning.skip_subtrees && tuning.prefilter,
            parsed: AtomicUsize::new(0),
            skipped: AtomicUsize::new(0),
            region_files: AtomicUsize::new(0),
            region_fallbacks: AtomicUsize::new(0),
            region_suspects: AtomicUsize::new(0),
            anchored_files: AtomicUsize::new(0),
            parsed_bytes: AtomicUsize::new(0),
            file_bytes: AtomicUsize::new(0),
            rule_timings: None,
        }
    }

    /// Starts timing each rule (`rule_times`). Slows matching down: leave it off unless asked.
    pub fn time_rules(&mut self) {
        self.rule_timings = Some(rule_timing::RuleTimings::new(self.rules.len()));
    }

    /// What each rule cost so far; empty unless `time_rules` was called.
    pub fn rule_times(&self) -> Vec<RuleTime> {
        let Some(timings) = &self.rule_timings else {
            return Vec::new();
        };
        self.rules
            .iter()
            .enumerate()
            .map(|(index, rule)| {
                let (nanos, files) = timings.total(index);
                RuleTime {
                    id: rule.id().to_owned(),
                    nanos,
                    files,
                }
            })
            .collect()
    }

    pub fn stats(&self) -> PrefilterStats {
        PrefilterStats {
            parsed: self.parsed.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
            region_files: self.region_files.load(Ordering::Relaxed),
            region_fallbacks: self.region_fallbacks.load(Ordering::Relaxed),
            region_suspects: self.region_suspects.load(Ordering::Relaxed),
            anchored_files: self.anchored_files.load(Ordering::Relaxed),
            parsed_bytes: self.parsed_bytes.load(Ordering::Relaxed),
            file_bytes: self.file_bytes.load(Ordering::Relaxed),
        }
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Whether some rule's language covers `path`, by ast-grep's extension mapping.
    pub fn covers(&self, path: &Path) -> bool {
        language_of(path).is_some_and(|language| self.groups.iter().any(|g| g.language == language))
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}
