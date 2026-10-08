mod chains;
mod prefilter;
mod regions;

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
    /// Every match lies inside one top-level statement and depends on nothing outside it.
    local: bool,
    /// Uses `has`: a match may be a container (e.g. a class) whose whole body matters, so the
    /// rule cannot be narrowed below top-level statements.
    inspects_descendants: bool,
    /// Uses `inside`.
    looks_at_ancestors: bool,
}

fn uses_key(value: &serde_json::Value, wanted: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .any(|(key, v)| (key == wanted && !v.is_null()) || uses_key(v, wanted)),
        serde_json::Value::Array(items) => items.iter().any(|v| uses_key(v, wanted)),
        _ => false,
    }
}

/// Whether a rule only looks at a node, its descendants and its ancestors: no siblings
/// (`follows`, `precedes`, `nthChild`), no utility rules it cannot see, no module-level match.
fn is_local(rule: &serde_json::Value) -> bool {
    match rule {
        serde_json::Value::Object(map) => map.iter().all(|(key, value)| match key.as_str() {
            _ if value.is_null() => true,
            "follows" | "precedes" | "nthChild" | "matches" => false,
            "kind" => value
                .as_str()
                .is_none_or(|k| k != "module" && k != "program"),
            _ => is_local(value),
        }),
        serde_json::Value::Array(items) => items.iter().all(is_local),
        _ => true,
    }
}

/// Whether this build carries the grammar for `language`. ast-grep panics when asked for a
/// grammar whose feature is off, so callers must check first.
pub fn compiled(language: SupportLang) -> bool {
    use SupportLang as L;
    match language {
        L::Bash
        | L::C
        | L::Css
        | L::Go
        | L::Hcl
        | L::Html
        | L::Java
        | L::JavaScript
        | L::Json
        | L::Lua
        | L::Python
        | L::Rust
        | L::Tsx
        | L::TypeScript
        | L::Yaml => true,
        L::Cpp => cfg!(feature = "lang-cpp"),
        L::CSharp => cfg!(feature = "lang-csharp"),
        L::Dart => cfg!(feature = "lang-dart"),
        L::Elixir => cfg!(feature = "lang-elixir"),
        L::Haskell => cfg!(feature = "lang-haskell"),
        L::Kotlin => cfg!(feature = "lang-kotlin"),
        L::Markdown => cfg!(feature = "lang-markdown"),
        L::Nix => cfg!(feature = "lang-nix"),
        L::Php => cfg!(feature = "lang-php"),
        L::Ruby => cfg!(feature = "lang-ruby"),
        L::Scala => cfg!(feature = "lang-scala"),
        L::Solidity => cfg!(feature = "lang-solidity"),
        L::Swift => cfg!(feature = "lang-swift"),
    }
}

/// The language a file is linted as, if its grammar is in this build.
fn language_of(path: &Path) -> Option<SupportLang> {
    SupportLang::from_path(path).filter(|&language| compiled(language))
}

/// Rejects rules for languages this build lacks, before ast-grep compiles their patterns.
fn check_languages(yaml: &str) -> Result<()> {
    #[derive(serde::Deserialize)]
    struct Head {
        language: Option<String>,
    }
    for document in serde_yaml::Deserializer::from_str(yaml) {
        let Ok(head) = <Head as serde::Deserialize>::deserialize(document) else {
            // ast-grep reports malformed rules itself. Stop here: after an error the
            // deserializer does not advance, so `continue` would loop forever.
            break;
        };
        let Some(name) = head.language else { continue };
        if let Ok(language) = name.parse::<SupportLang>() {
            anyhow::ensure!(
                compiled(language),
                "rule language `{name}` is not compiled into this botica build \
                 (enable the botica-astgrep feature for it, or `all-languages`)"
            );
        }
    }
    Ok(())
}

impl AstGrepRule {
    pub fn from_yaml(yaml: &str) -> Result<Vec<Self>> {
        check_languages(yaml)?;
        let globals = GlobalRules::default();
        from_yaml_string::<SupportLang>(yaml, &globals)
            .context("invalid ast-grep rule")?
            .into_iter()
            .map(|config| {
                let serialized = serde_json::to_value(&*config)?;
                let files = build_globs(&serialized, "files")?;
                let ignores = build_globs(&serialized, "ignores")?;
                let local = serialized.get("utils").is_none_or(|u| {
                    u.is_null() || u.as_object().is_some_and(serde_json::Map::is_empty)
                }) && serialized.get("rule").is_some_and(is_local);
                let inspects_descendants = serialized
                    .get("rule")
                    .is_some_and(|rule| uses_key(rule, "has"));
                let looks_at_ancestors = serialized
                    .get("rule")
                    .is_some_and(|rule| uses_key(rule, "inside"));
                Ok(Self {
                    looks_at_ancestors,
                    inspects_descendants,
                    local,
                    config,
                    files,
                    ignores,
                    requirement: prefilter::Req::Always,
                    declared: None,
                    rule_json: serialized.get("rule").cloned().unwrap_or_default(),
                })
            })
            .collect()
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    /// Declares the regex the file text must match (the `prefilter` field of `rule.toml`), in
    /// place of the one derived from the rule. Only rules without a pattern need it (`kind:`
    /// only). The caller vouches that every match of the rule implies a match of the regex.
    pub fn declare_prefilter(&mut self, pattern: &str) {
        self.declared = Some(prefilter::Req::Re(pattern.to_owned()));
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
fn parse_reusing(
    lang: SupportLang,
    text: &str,
    ranges: &[tree_sitter::Range],
) -> Option<AstGrep<StrDoc<SupportLang>>> {
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
        let parser = &mut parsers[position].1;
        if ranges.is_empty() {
            return parser.parse(text.as_bytes(), None);
        }
        parser.set_included_ranges(ranges).ok()?;
        let tree = parser.parse(text.as_bytes(), None);
        parser.set_included_ranges(&[]).ok()?;
        tree
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
    /// Files parsed by regions, files whose partial tree had errors (re-parsed whole), and the
    /// bytes handed to the parser against the bytes of the parsed files.
    pub region_files: usize,
    pub region_fallbacks: usize,
    /// Files the lexer flagged as likely syntax errors, parsed whole to match ast-grep.
    pub region_suspects: usize,
    pub parsed_bytes: usize,
    pub file_bytes: usize,
    /// CPU time per phase, summed over threads, indexed by the `PHASE_*` constants.
    pub phase_nanos: [u64; 8],
}

/// Prefilter scan. Region selection, including the parse inside it (`PHASE_PARSE_REGIONS`).
/// Whole-file parse. Parse of the selected regions. Rule matching. Freeing the tree.
pub const PHASE_PREFILTER: usize = 0;
pub const PHASE_REGIONS: usize = 1;
pub const PHASE_PARSE: usize = 2;
pub const PHASE_PARSE_REGIONS: usize = 3;
pub const PHASE_MATCH: usize = 4;
pub const PHASE_DROP: usize = 5;
/// Inside `PHASE_REGIONS`: top-level lexing, and narrowing to class/function members.
pub const PHASE_LEX: usize = 6;
pub const PHASE_MEMBERS: usize = 7;

pub struct AstGrepSet {
    rules: Vec<AstGrepRule>,
    groups: Vec<LanguageGroup>,
    literals: HashMap<String, Finder<'static>>,
    regexes: HashMap<String, regex::bytes::Regex>,
    prefilter: bool,
    regions: bool,
    parsed: AtomicUsize,
    skipped: AtomicUsize,
    region_files: AtomicUsize,
    region_fallbacks: AtomicUsize,
    region_suspects: AtomicUsize,
    parsed_bytes: AtomicUsize,
    file_bytes: AtomicUsize,
    phase_nanos: [std::sync::atomic::AtomicU64; 8],
}

impl Default for AstGrepSet {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl AstGrepSet {
    /// Groups the rules by language and derives each rule's prefilter from the rule itself
    /// (token chains of its patterns, literals of its regexes); declared prefilters win.
    pub fn new(mut rules: Vec<AstGrepRule>) -> Self {
        let mut groups: Vec<LanguageGroup> = Vec::new();
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
        }
        for group in &mut groups {
            let mut table = chains::LangChains::new(group.language);
            for &index in &group.rules {
                let rule = &mut rules[index];
                rule.requirement = match &rule.declared {
                    Some(declared) => declared.clone(),
                    None => table.requirement(&rule.rule_json),
                };
            }
            table.finish();
            group.chains = (!table.is_empty()).then_some(table);
        }
        let mut literals: HashMap<String, Finder<'static>> = HashMap::new();
        let mut regexes: HashMap<String, regex::bytes::Regex> = HashMap::new();
        for rule in &rules {
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
            regions: true,
            parsed: AtomicUsize::new(0),
            skipped: AtomicUsize::new(0),
            region_files: AtomicUsize::new(0),
            region_fallbacks: AtomicUsize::new(0),
            region_suspects: AtomicUsize::new(0),
            parsed_bytes: AtomicUsize::new(0),
            file_bytes: AtomicUsize::new(0),
            phase_nanos: Default::default(),
        }
    }

    fn timed<T>(&self, phase: usize, work: impl FnOnce() -> T) -> T {
        let start = std::time::Instant::now();
        let out = work();
        let nanos = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.phase_nanos[phase].fetch_add(nanos, Ordering::Relaxed);
        out
    }

    /// Parses only the top-level statements holding a prefilter candidate (Python, JS/TS),
    /// on by default; off parses whole files, for parity checks.
    pub fn set_regions(&mut self, enabled: bool) {
        self.regions = enabled;
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

    /// Turns the prefilter on or off (on by default); off parses every file, for parity checks.
    pub fn set_prefilter(&mut self, enabled: bool) {
        self.prefilter = enabled;
    }

    pub fn stats(&self) -> PrefilterStats {
        PrefilterStats {
            parsed: self.parsed.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed),
            region_files: self.region_files.load(Ordering::Relaxed),
            region_fallbacks: self.region_fallbacks.load(Ordering::Relaxed),
            region_suspects: self.region_suspects.load(Ordering::Relaxed),
            parsed_bytes: self.parsed_bytes.load(Ordering::Relaxed),
            file_bytes: self.file_bytes.load(Ordering::Relaxed),
            phase_nanos: std::array::from_fn(|i| self.phase_nanos[i].load(Ordering::Relaxed)),
        }
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Which rules can match `text`, and the verified chain occurrences `(chain, offset)` found
    /// on the way (all of them, for region parsing; empty when no rule needs a chain).
    fn active_rules(
        &self,
        group: &LanguageGroup,
        path: &Path,
        text: &str,
    ) -> (Vec<bool>, Vec<(usize, usize)>) {
        let mut flags = vec![false; self.rules.len()];
        let mut seen: HashMap<String, bool> = HashMap::new();
        let mut chain_hits: Option<chains::ChainScan> = None;
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
                            return chain_hits
                                .get_or_insert_with(|| table.scan_all(text.as_bytes()))
                                .0[*i];
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
        (flags, chain_hits.map(|(_, at)| at).unwrap_or_default())
    }

    pub fn check(&self, path: &Path, text: &str) -> Vec<Hit> {
        let Some(lang) = language_of(path) else {
            return Vec::new();
        };
        let Some(group) = self.groups.iter().find(|g| g.language == lang) else {
            return Vec::new();
        };
        let (active, chain_at) =
            self.timed(PHASE_PREFILTER, || self.active_rules(group, path, text));
        if !active.iter().any(|&on| on) {
            self.skipped.fetch_add(1, Ordering::Relaxed);
            return Vec::new();
        }
        self.parsed.fetch_add(1, Ordering::Relaxed);
        self.file_bytes.fetch_add(text.len(), Ordering::Relaxed);
        let partial = if self.regions {
            self.timed(PHASE_REGIONS, || {
                self.parse_regions(lang, group, &active, &chain_at, text)
            })
        } else {
            None
        };
        let grep = partial.or_else(|| {
            self.parsed_bytes.fetch_add(text.len(), Ordering::Relaxed);
            self.timed(PHASE_PARSE, || parse_reusing(lang, text, &[]))
        });
        match grep {
            Some(grep) => {
                let hits = self.timed(PHASE_MATCH, || self.run(group, &active, &grep));
                self.timed(PHASE_DROP, || drop(grep));
                hits
            }
            None => Vec::new(),
        }
    }

    /// Parses only the top-level statements that hold a candidate position of an active rule,
    /// or returns `None` when that is not safe or not worth it.
    fn parse_regions(
        &self,
        lang: SupportLang,
        group: &LanguageGroup,
        active: &[bool],
        chain_at: &[(usize, usize)],
        text: &str,
    ) -> Option<AstGrep<StrDoc<SupportLang>>> {
        let bytes = text.as_bytes();
        let split = self.timed(PHASE_LEX, || regions::chunks(lang, bytes))?;
        if split.suspect {
            self.region_suspects.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let chunks = split.chunks;
        let mut wanted = vec![false; group.chains.as_ref().map_or(0, chains::LangChains::len)];
        let mut positions = Vec::new();
        let mut members = true;
        for &index in &group.rules {
            if !active[index] {
                continue;
            }
            let rule = &self.rules[index];
            if !rule.local || rule.requirement == prefilter::Req::Always {
                return None;
            }
            members &= !rule.inspects_descendants;
            let mut unknown = false;
            let mut visit = |atom: &prefilter::Req| match atom {
                prefilter::Req::Chain(i) => wanted[*i] = true,
                prefilter::Req::Lit(literal) => match self.literals.get(literal) {
                    Some(finder) => positions.extend(finder.find_iter(bytes)),
                    None => unknown = true,
                },
                prefilter::Req::Re(pattern) => match self.regexes.get(pattern) {
                    Some(re) => positions.extend(re.find_iter(bytes).map(|m| m.start())),
                    None => unknown = true,
                },
                _ => unknown = true,
            };
            // An `inside` part lies in an ancestor, maybe in another class member than the
            // match: every part must then mark its member.
            if rule.looks_at_ancestors {
                rule.requirement.each_atom(&mut visit);
            } else {
                rule.requirement.each_first_atom(&mut visit);
            }
            if unknown {
                return None;
            }
        }
        positions.extend(
            chain_at
                .iter()
                .filter(|&&(chain, _)| wanted[chain])
                .map(|&(_, at)| at),
        );
        let top = regions::pick(&chunks, &mut positions);
        let picked = if members {
            self.timed(PHASE_MEMBERS, || {
                regions::members(lang, bytes, &top, &positions)
            })
        } else {
            regions::merge(top)
        };
        let picked_bytes: usize = picked.iter().map(|c| c.end - c.start).sum();
        if picked.is_empty() || picked_bytes * 5 > text.len() * 4 {
            return None;
        }
        let last_line = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
        let point = |offset: usize, row: usize| tree_sitter::Point {
            row,
            column: if offset == bytes.len() {
                offset - last_line
            } else {
                0
            },
        };
        let ranges: Vec<tree_sitter::Range> = picked
            .iter()
            .map(|c| tree_sitter::Range {
                start_byte: c.start,
                end_byte: c.end,
                start_point: point(c.start, c.start_row),
                end_point: point(c.end, c.end_row),
            })
            .collect();
        let grep = self.timed(PHASE_PARSE_REGIONS, || parse_reusing(lang, text, &ranges))?;
        if grep.root().get_inner_node().has_error() {
            self.region_fallbacks.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        self.region_files.fetch_add(1, Ordering::Relaxed);
        self.parsed_bytes.fetch_add(picked_bytes, Ordering::Relaxed);
        Some(grep)
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
        let language = language_of(path)?;
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

#[cfg(test)]
mod language_tests {
    use super::*;

    #[test]
    fn base_languages_are_compiled() {
        for language in [
            SupportLang::Python,
            SupportLang::TypeScript,
            SupportLang::Go,
        ] {
            assert!(compiled(language));
        }
    }

    #[test]
    fn region_parsing_finds_the_same_hits_as_a_whole_file_parse() {
        let yaml = "id: p\nlanguage: python\nrule:\n  pattern: print($$$A)\n  not:\n    inside:\n      stopBy: end\n      pattern: 'if __name__ == \"__main__\": $$$B'\n";
        let source = "import os\n\nX = 1\n\ndef f():\n    print(1)\n\nclass C:\n    def g(self):\n        '''\nprint(2)\n'''\n        return 2\n\nY = [\nprint(3)]\n\nif __name__ == \"__main__\":\n    print(4)\n";
        let filler: String = (0..40)
            .map(|i| format!("def h{i}(a, b):\n    return a + b * {i}\n\n"))
            .collect();
        let source = format!("{filler}{source}");
        let source = source.as_str();
        let mut whole = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
        whole.set_regions(false);
        let regions = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
        let path = Path::new("m.py");
        let expected = whole.check(path, source);
        assert_eq!(expected.len(), 2);
        assert_eq!(regions.check(path, source), expected);
        assert_eq!(regions.stats().region_files, 1);
        assert!(regions.stats().parsed_bytes < source.len());
    }

    #[test]
    fn malformed_yaml_is_an_error_not_a_hang() {
        let yaml = "id: x\nlanguage: python\nrule:\n  pattern: if a: b: c\n";
        assert!(AstGrepRule::from_yaml(yaml).is_err());
    }

    #[test]
    fn rules_that_look_at_siblings_are_not_local() {
        let local =
            AstGrepRule::from_yaml("id: a\nlanguage: python\nrule:\n  pattern: print($A)\n")
                .unwrap();
        assert!(local[0].local);
        let sibling = AstGrepRule::from_yaml(
            "id: b\nlanguage: python\nrule:\n  pattern: print($A)\n  follows:\n    pattern: import os\n",
        )
        .unwrap();
        assert!(!sibling[0].local);
    }

    #[cfg(not(feature = "lang-kotlin"))]
    #[test]
    fn rules_for_missing_grammars_fail_cleanly_and_their_files_are_skipped() {
        let error =
            AstGrepRule::from_yaml("id: k\nlanguage: kotlin\nrule:\n  pattern: println($A)\n")
                .err()
                .expect("kotlin is not compiled in");
        assert!(error
            .to_string()
            .contains("not compiled into this botica build"));
        let rules =
            AstGrepRule::from_yaml("id: p\nlanguage: python\nrule:\n  pattern: print($A)\n")
                .expect("python is base");
        let set = AstGrepSet::new(rules);
        assert!(set
            .check(Path::new("Main.kt"), "fun main() { println(1) }")
            .is_empty());
        assert_eq!(set.check(Path::new("m.py"), "print(1)\n").len(), 1);
    }
}
