mod chains;
mod prefilter;

use anyhow::{Context, Result};
use ast_grep_config::{from_yaml_string, GlobalRules, RuleConfig, Severity};
use ast_grep_core::matcher::MatcherExt;
use ast_grep_core::source::Edit;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::{AstGrep, Matcher};
use ast_grep_language::{Language, LanguageExt, SupportLang};
use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use memchr::memmem::Finder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub rule_id: String,
    pub severity: &'static str,
    pub start: usize,
    pub end: usize,
    pub message: String,
}

pub struct AstGrepRule {
    config: RuleConfig<SupportLang>,
    files: Option<GlobSet>,
    ignores: Option<GlobSet>,
    requirement: prefilter::Req,
    /// A hand-written requirement (`prefilter` in `rule.toml`); wins over the derived one.
    declared: Option<prefilter::Req>,
    rule_json: serde_json::Value,
}

impl AstGrepRule {
    pub fn from_yaml(yaml: &str) -> Result<Vec<Self>> {
        let globals = GlobalRules::default();
        from_yaml_string::<SupportLang>(yaml, &globals)
            .context("invalid ast-grep rule")?
            .into_iter()
            .map(|config| {
                let serialized = serde_json::to_value(&*config)?;
                let files = build_globs(&serialized, "files")?;
                let ignores = build_globs(&serialized, "ignores")?;
                let requirement = serialized
                    .get("rule")
                    .map_or(prefilter::Req::Always, prefilter::requirement);
                Ok(Self {
                    config,
                    files,
                    ignores,
                    requirement,
                    declared: None,
                    rule_json: serialized.get("rule").cloned().unwrap_or_default(),
                })
            })
            .collect()
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    /// Replaces the requirement with a hand-declared regex the file text must match. The caller
    /// vouches that every match of the rule implies a match of the regex.
    pub fn require_regex(&mut self, pattern: &str) {
        self.requirement = prefilter::Req::Re(pattern.to_owned());
        self.declared = Some(self.requirement.clone());
    }

    /// Declares literals the file must contain for this rule to match (for rules that
    /// ast-grep expresses without a pattern, e.g. `kind:` only). The caller vouches for them.
    pub fn require(&mut self, literals: &[&str]) {
        let extra = literals
            .iter()
            .map(|l| prefilter::Req::Lit((*l).to_owned()))
            .collect();
        let current = std::mem::replace(&mut self.requirement, prefilter::Req::Always);
        self.requirement = prefilter::Req::And(vec![current, prefilter::Req::And(extra)]);
    }

    pub fn language(&self) -> SupportLang {
        self.config.language
    }

    fn message<D: ast_grep_core::Doc>(&self, found: &ast_grep_core::NodeMatch<'_, D>) -> String {
        self.config.get_message(found)
    }

    pub fn applies_to(&self, path: &Path) -> bool {
        let included = self.files.as_ref().is_none_or(|g| g.is_match(path));
        let excluded = self.ignores.as_ref().is_some_and(|g| g.is_match(path));
        included && !excluded
    }
}

/// `RuleFileGlob` is private in ast-grep-config 0.41, so `files`/`ignores` are read back
/// through the config's `Serialize` impl.
fn build_globs(config: &serde_json::Value, key: &str) -> Result<Option<GlobSet>> {
    let Some(entries) = config.get(key).and_then(serde_json::Value::as_array) else {
        return Ok(None);
    };
    let mut builder = GlobSetBuilder::new();
    for entry in entries {
        match entry {
            serde_json::Value::String(pattern) => {
                builder.add(Glob::new(pattern)?);
            }
            serde_json::Value::Object(map) => {
                let pattern = map.get("glob").and_then(|g| g.as_str()).unwrap_or_default();
                let insensitive = map
                    .get("caseInsensitive")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                builder.add(
                    GlobBuilder::new(pattern)
                        .case_insensitive(insensitive)
                        .build()?,
                );
            }
            _ => {}
        }
    }
    Ok(Some(builder.build()?))
}

thread_local! {
    static PARSERS: std::cell::RefCell<Vec<(SupportLang, tree_sitter::Parser)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// ast-grep 0.41 builds a `Parser` and calls `set_language` for every file; this keeps one
/// per thread and language.
fn parse_reusing(lang: SupportLang, text: &str) -> Option<AstGrep<StrDoc<SupportLang>>> {
    let tree = PARSERS.with(|cell| {
        let mut parsers = cell.borrow_mut();
        let position = match parsers.iter().position(|(l, _)| *l == lang) {
            Some(position) => position,
            None => {
                let mut parser = tree_sitter::Parser::new();
                parser.set_language(&lang.get_ts_language()).ok()?;
                parsers.push((lang, parser));
                parsers.len() - 1
            }
        };
        parsers[position].1.parse(text.as_bytes(), None)
    })?;
    Some(AstGrep::doc(StrDoc {
        src: text.to_owned(),
        lang,
        tree,
    }))
}

fn severity_name(severity: &Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint | Severity::Off => "hint",
    }
}

/// Rules of one language, indexed by the node kinds they can match, so a file is walked once
/// no matter how many rules apply (the same idea as ast-grep's own `CombinedScan`).
struct LanguageGroup {
    language: SupportLang,
    rules: Vec<usize>,
    by_kind: Vec<Vec<usize>>,
    chains: Option<chains::LangChains>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PrefilterStats {
    pub parsed: usize,
    pub skipped: usize,
}

pub struct AstGrepSet {
    rules: Vec<AstGrepRule>,
    groups: Vec<LanguageGroup>,
    literals: HashMap<String, Finder<'static>>,
    regexes: HashMap<String, regex::bytes::Regex>,
    prefilter: bool,
    parsed: AtomicUsize,
    skipped: AtomicUsize,
}

impl Default for AstGrepSet {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl AstGrepSet {
    pub fn new(rules: Vec<AstGrepRule>) -> Self {
        let mut groups: Vec<LanguageGroup> = Vec::new();
        let mut literals: HashMap<String, Finder<'static>> = HashMap::new();
        let mut regexes: HashMap<String, regex::bytes::Regex> = HashMap::new();
        for (index, rule) in rules.iter().enumerate() {
            let language = rule.language();
            let position = groups.iter().position(|g| g.language == language);
            let group = match position {
                Some(position) => &mut groups[position],
                None => {
                    groups.push(LanguageGroup {
                        language,
                        rules: Vec::new(),
                        by_kind: Vec::new(),
                        chains: None,
                    });
                    groups.last_mut().expect("just pushed")
                }
            };
            group.rules.push(index);
            if let Some(kinds) = rule.config.matcher.potential_kinds() {
                for kind in kinds.iter() {
                    if group.by_kind.len() <= kind {
                        group.by_kind.resize(kind + 1, Vec::new());
                    }
                    group.by_kind[kind].push(index);
                }
            }
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
        Self {
            rules,
            groups,
            literals,
            regexes,
            prefilter: true,
            parsed: AtomicUsize::new(0),
            skipped: AtomicUsize::new(0),
        }
    }

    /// Derives each rule's requirement from the rule itself (token chains of its patterns,
    /// literals of its regexes) instead of loose identifiers; hand-declared ones still win.
    pub fn use_auto_prefilter(&mut self) {
        for group in &mut self.groups {
            let mut table = chains::LangChains::new(group.language);
            for &index in &group.rules {
                let rule = &mut self.rules[index];
                rule.requirement = match &rule.declared {
                    Some(declared) => declared.clone(),
                    None => table.requirement(&rule.rule_json),
                };
                let mut needed = Vec::new();
                rule.requirement.literals(&mut needed);
                for literal in needed {
                    self.literals
                        .entry(literal.to_owned())
                        .or_insert_with(|| Finder::new(literal.as_bytes()).into_owned());
                }
                let mut patterns = Vec::new();
                rule.requirement.regexes(&mut patterns);
                for pattern in patterns {
                    self.regexes.entry(pattern.to_owned()).or_insert_with(|| {
                        regex::bytes::Regex::new(pattern).expect("valid prefilter regex")
                    });
                }
            }
            table.finish();
            group.chains = (!table.is_empty()).then_some(table);
        }
    }

    /// Each rule's id with its effective requirement, for inspection.
    pub fn describe_prefilter(&self) -> Vec<(String, String)> {
        let chains = self
            .groups
            .iter()
            .filter_map(|g| g.chains.as_ref())
            .flat_map(chains::LangChains::describe)
            .map(|line| ("chain".to_owned(), line));
        self.rules
            .iter()
            .map(|r| (r.id().to_owned(), format!("{:?}", r.requirement)))
            .chain(chains)
            .collect()
    }

    /// Turns the required-literal prefilter on or off (on by default).
    pub fn set_prefilter(&mut self, enabled: bool) {
        self.prefilter = enabled;
    }

    pub fn stats(&self) -> PrefilterStats {
        PrefilterStats {
            parsed: self.parsed.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
        }
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    fn active_rules(&self, group: &LanguageGroup, path: &Path, text: &str) -> Vec<bool> {
        let mut flags = vec![false; self.rules.len()];
        let mut seen: HashMap<String, bool> = HashMap::new();
        let mut chain_hits: Option<Vec<bool>> = None;
        for &index in &group.rules {
            if !self.rules[index].applies_to(path) {
                continue;
            }
            flags[index] = !self.prefilter
                || self.rules[index].requirement.holds(&mut |atom| {
                    let key = match atom {
                        prefilter::Req::Lit(s) | prefilter::Req::Re(s) => s,
                        prefilter::Req::Chain(i) => {
                            let Some(table) = &group.chains else {
                                return true;
                            };
                            return chain_hits.get_or_insert_with(|| table.scan(text.as_bytes()))
                                [*i];
                        }
                        _ => return true,
                    };
                    if let Some(&known) = seen.get(key) {
                        return known;
                    }
                    let found = match atom {
                        prefilter::Req::Lit(literal) => self
                            .literals
                            .get(literal)
                            .is_none_or(|finder| finder.find(text.as_bytes()).is_some()),
                        _ => self
                            .regexes
                            .get(key)
                            .is_none_or(|re| re.is_match(text.as_bytes())),
                    };
                    seen.insert(key.to_owned(), found);
                    found
                });
        }
        flags
    }

    pub fn check(&self, path: &Path, text: &str) -> Vec<Hit> {
        let Some(lang) = SupportLang::from_path(path) else {
            return Vec::new();
        };
        let Some(group) = self.groups.iter().find(|g| g.language == lang) else {
            return Vec::new();
        };
        let active = self.active_rules(group, path, text);
        if !active.iter().any(|&on| on) {
            self.skipped.fetch_add(1, Ordering::Relaxed);
            return Vec::new();
        }
        self.parsed.fetch_add(1, Ordering::Relaxed);
        match parse_reusing(lang, text) {
            Some(grep) => self.run(group, &active, &grep),
            None => Vec::new(),
        }
    }

    fn run(
        &self,
        group: &LanguageGroup,
        active: &[bool],
        grep: &AstGrep<StrDoc<SupportLang>>,
    ) -> Vec<Hit> {
        let mut hits = Vec::new();
        for node in grep.root().dfs() {
            let Some(candidates) = group.by_kind.get(node.kind_id() as usize) else {
                continue;
            };
            for &index in candidates {
                if !active[index] {
                    continue;
                }
                let rule = &self.rules[index];
                let Some(found) = rule.config.matcher.match_node(node.clone()) else {
                    continue;
                };
                let range = found.range();
                hits.push(Hit {
                    rule_id: rule.id().to_owned(),
                    severity: severity_name(&rule.config.severity),
                    start: range.start,
                    end: range.end,
                    message: rule.message(&found),
                });
            }
        }
        hits
    }

    /// Opens an editable buffer (LSP): the syntax tree is kept and re-used between edits.
    pub fn open(&self, path: &Path, text: &str) -> Option<Buffer> {
        let language = SupportLang::from_path(path)?;
        self.groups.iter().find(|g| g.language == language)?;
        Some(Buffer {
            path: path.to_path_buf(),
            language,
            grep: language.ast_grep(text),
        })
    }

    pub fn check_buffer(&self, buffer: &Buffer) -> Vec<Hit> {
        let Some(group) = self.groups.iter().find(|g| g.language == buffer.language) else {
            return Vec::new();
        };
        let mut active = vec![false; self.rules.len()];
        for &index in &group.rules {
            active[index] = self.rules[index].applies_to(&buffer.path);
        }
        self.run(group, &active, &buffer.grep)
    }
}

pub struct Buffer {
    path: PathBuf,
    language: SupportLang,
    grep: AstGrep<StrDoc<SupportLang>>,
}

impl Buffer {
    pub fn text(&self) -> &str {
        &self.grep.root().get_doc().src
    }

    /// Replaces `deleted` bytes at `start` with `inserted`, re-parsing incrementally.
    pub fn edit(&mut self, start: usize, deleted: usize, inserted: &str) -> Result<()> {
        self.grep
            .edit(Edit {
                position: start,
                deleted_length: deleted,
                inserted_text: inserted.as_bytes().to_vec(),
            })
            .map_err(anyhow::Error::msg)?;
        Ok(())
    }

    pub fn language(&self) -> SupportLang {
        self.language
    }
}
