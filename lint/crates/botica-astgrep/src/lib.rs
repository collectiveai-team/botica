mod chains;
mod prefilter;
mod regions;
mod rule_timing;

pub use rule_timing::RuleTime;

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
    /// Reads the contents of a container, so the rule cannot be narrowed below top-level
    /// statements: it uses `has` (the match may be a class whose whole body matters), or an
    /// `inside` that reads its ancestor's text or descendants (a partly parsed class would not
    /// match it, and a `not` around it would then report a false hit).
    inspects_descendants: bool,
    /// Uses `inside` outside a `not`: part of its prefilter lies in an ancestor of the match.
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

/// Whether an `inside` (under a `not` or not) reads the contents of an ancestor that member
/// narrowing may leave partly parsed: it uses `has` or `regex`, or a `pattern` that can match
/// such a container. In Python only top-level `class`/`def` bodies are narrowed, so a pattern
/// rooted elsewhere (e.g. `if __name__ == "__main__": …`) is safe; in JS/TS any statement ending
/// in a block may be narrowed, so every pattern counts.
fn inside_reads_contents(value: &serde_json::Value, language: SupportLang) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, v)| {
            let reads = key == "inside"
                && (uses_key(v, "has")
                    || uses_key(v, "regex")
                    || any_value_of(v, "pattern", &|p| !spares_members(p, language)));
            reads || inside_reads_contents(v, language)
        }),
        serde_json::Value::Array(items) => items.iter().any(|v| inside_reads_contents(v, language)),
        _ => false,
    }
}

/// Whether some value under key `wanted`, at any depth, satisfies `test`.
fn any_value_of(
    value: &serde_json::Value,
    wanted: &str,
    test: &dyn Fn(&serde_json::Value) -> bool,
) -> bool {
    match value {
        serde_json::Value::Object(map) => map
            .iter()
            .any(|(key, v)| (key == wanted && test(v)) || any_value_of(v, wanted, test)),
        serde_json::Value::Array(items) => items.iter().any(|v| any_value_of(v, wanted, test)),
        _ => false,
    }
}

/// Whether a pattern can never match a node that member narrowing leaves partly parsed.
fn spares_members(pattern: &serde_json::Value, language: SupportLang) -> bool {
    const PYTHON_CONTAINERS: [&str; 4] = [
        "class_definition",
        "function_definition",
        "decorated_definition",
        "block",
    ];
    let (Some(source), SupportLang::Python) = (pattern.as_str(), language) else {
        return false;
    };
    let Some(kinds) = ast_grep_core::Pattern::try_new(source, language)
        .ok()
        .and_then(|p| p.potential_kinds())
    else {
        return false;
    };
    PYTHON_CONTAINERS
        .iter()
        .all(|kind| !kinds.contains(usize::from(language.kind_to_id(kind))))
}

/// Like `uses_key`, but not under a `not`: the prefilter takes no atom from there.
fn uses_key_outside_not(value: &serde_json::Value, wanted: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => map.iter().any(|(key, v)| {
            key != "not" && ((key == wanted && !v.is_null()) || uses_key_outside_not(v, wanted))
        }),
        serde_json::Value::Array(items) => items.iter().any(|v| uses_key_outside_not(v, wanted)),
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
                let inspects_descendants = serialized.get("rule").is_some_and(|rule| {
                    uses_key(rule, "has") || inside_reads_contents(rule, config.language)
                });
                let looks_at_ancestors = serialized
                    .get("rule")
                    .is_some_and(|rule| uses_key_outside_not(rule, "inside"));
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

/// Where the active rules of a file can match, from their prefilter atoms.
struct Candidates {
    /// Sorted, without duplicates.
    positions: Vec<usize>,
    /// No active rule uses `has`, so regions may narrow to class/function members.
    members: bool,
    /// Every match of every active rule contains one of `positions`, so matching may skip
    /// the subtrees that hold none: no rule uses `inside` or a hand-written regex.
    anchored: bool,
}

pub struct AstGrepSet {
    rules: Vec<AstGrepRule>,
    groups: Vec<LanguageGroup>,
    literals: HashMap<String, Finder<'static>>,
    regexes: HashMap<String, regex::bytes::Regex>,
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
        let (active, chain_at) = self.active_rules(group, path, text);
        if !active.iter().any(|&on| on) {
            self.skipped.fetch_add(1, Ordering::Relaxed);
            return Vec::new();
        }
        self.parsed.fetch_add(1, Ordering::Relaxed);
        self.file_bytes.fetch_add(text.len(), Ordering::Relaxed);
        let candidates = if self.prefilter && (self.regions || self.skip_subtrees) {
            self.candidates(group, &active, &chain_at, text.as_bytes())
        } else {
            None
        };
        let partial = match &candidates {
            Some(candidates) if self.regions => self.parse_regions(lang, candidates, text),
            _ => None,
        };
        let grep = partial.or_else(|| {
            self.parsed_bytes.fetch_add(text.len(), Ordering::Relaxed);
            parse_reusing(lang, text, &[])
        });
        let anchor = candidates
            .as_ref()
            .filter(|c| self.skip_subtrees && c.anchored)
            .map(|c| c.positions.as_slice());
        if anchor.is_some() {
            self.anchored_files.fetch_add(1, Ordering::Relaxed);
        }
        match grep {
            Some(grep) => self.run(group, &active, &grep, anchor),
            None => Vec::new(),
        }
    }

    /// The candidate positions of the active rules, or `None` when some active rule cannot be
    /// located by them: it is not local, its prefilter is `Always`, or an atom is unknown.
    fn candidates(
        &self,
        group: &LanguageGroup,
        active: &[bool],
        chain_at: &[(usize, usize)],
        bytes: &[u8],
    ) -> Option<Candidates> {
        let chain_count = group.chains.as_ref().map_or(0, chains::LangChains::len);
        let mut wanted = vec![false; chain_count];
        let mut occurrences = vec![0usize; chain_count];
        for &(chain, _) in chain_at {
            occurrences[chain] += 1;
        }
        let mut found: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut positions = Vec::new();
        let mut members = true;
        let mut anchored = true;
        for &index in &group.rules {
            if !active[index] {
                continue;
            }
            let rule = &self.rules[index];
            if !rule.local || rule.requirement == prefilter::Req::Always {
                return None;
            }
            members &= !rule.inspects_descendants;
            anchored &= !rule.looks_at_ancestors;
            let mut unknown = false;
            let mut count = |atom: &prefilter::Req| -> usize {
                match atom {
                    prefilter::Req::Chain(i) => occurrences[*i],
                    prefilter::Req::Lit(literal) => match self.literals.get_key_value(literal) {
                        Some((key, finder)) => found
                            .entry(key.as_str())
                            .or_insert_with(|| finder.find_iter(bytes).collect())
                            .len(),
                        None => {
                            unknown = true;
                            0
                        }
                    },
                    prefilter::Req::Re(pattern) => match self.regexes.get_key_value(pattern) {
                        Some((key, re)) => found
                            .entry(key.as_str())
                            .or_insert_with(|| re.find_iter(bytes).map(|m| m.start()).collect())
                            .len(),
                        None => {
                            unknown = true;
                            0
                        }
                    },
                    _ => {
                        unknown = true;
                        0
                    }
                }
            };
            let mut atoms = Vec::new();
            // An `inside` part lies in an ancestor, maybe in another class member than the
            // match: every part must then mark its member.
            if rule.looks_at_ancestors {
                rule.requirement.each_atom(&mut |atom| {
                    count(atom);
                    atoms.push(atom.clone());
                });
            } else {
                let mut rarest = Vec::new();
                rule.requirement.rarest_atoms(&mut count, &mut rarest);
                atoms.extend(rarest.into_iter().cloned());
            }
            if unknown {
                return None;
            }
            for atom in &atoms {
                match atom {
                    prefilter::Req::Chain(i) => wanted[*i] = true,
                    prefilter::Req::Lit(key) => positions.extend_from_slice(&found[key.as_str()]),
                    // A hand-written regex may start before the node it stands for.
                    prefilter::Req::Re(key) => {
                        anchored = false;
                        positions.extend_from_slice(&found[key.as_str()]);
                    }
                    _ => {}
                }
            }
        }
        positions.extend(
            chain_at
                .iter()
                .filter(|&&(chain, _)| wanted[chain])
                .map(|&(_, at)| at),
        );
        positions.sort_unstable();
        positions.dedup();
        Some(Candidates {
            positions,
            members,
            anchored,
        })
    }

    /// Parses only the top-level statements that hold a candidate position of an active rule,
    /// or returns `None` when that is not safe or not worth it.
    fn parse_regions(
        &self,
        lang: SupportLang,
        candidates: &Candidates,
        text: &str,
    ) -> Option<AstGrep<StrDoc<SupportLang>>> {
        let bytes = text.as_bytes();
        let split = regions::chunks(lang, bytes)?;
        if split.suspect {
            self.region_suspects.fetch_add(1, Ordering::Relaxed);
            return None;
        }
        let positions = &candidates.positions;
        let top = regions::pick(&split.chunks, positions);
        let picked = if candidates.members {
            regions::members(lang, bytes, &top, positions)
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
        let grep = parse_reusing(lang, text, &ranges)?;
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
        anchor: Option<&[usize]>,
    ) -> Vec<Hit> {
        let mut hits = Vec::new();
        let mut spent = self
            .rule_timings
            .as_ref()
            .map(|_| vec![0u64; self.rules.len()]);
        let visit = |node: ast_grep_core::Node<'_, StrDoc<SupportLang>>| {
            let Some(candidates) = group.by_kind.get(node.kind_id() as usize) else {
                return;
            };
            for &index in candidates {
                if !active[index] {
                    continue;
                }
                let rule = &self.rules[index];
                let matched = match spent.as_mut() {
                    Some(spent) => {
                        let start = std::time::Instant::now();
                        let matched = rule.config.matcher.match_node(node.clone());
                        spent[index] +=
                            u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
                        matched
                    }
                    None => rule.config.matcher.match_node(node.clone()),
                };
                let Some(found) = matched else {
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
        };
        match anchor {
            Some(positions) => each_node_holding(grep, positions, visit),
            None => grep.root().dfs().for_each(visit),
        }
        if let (Some(timings), Some(spent)) = (&self.rule_timings, &spent) {
            timings.add(spent, active);
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
        self.run(group, &active, &buffer.grep, None)
    }
}

/// Visits, in pre-order, the nodes whose byte range holds one of `positions` (sorted). A node
/// holding none has no descendant holding one, so its whole subtree is skipped.
fn each_node_holding<'r>(
    grep: &'r AstGrep<StrDoc<SupportLang>>,
    positions: &[usize],
    mut visit: impl FnMut(ast_grep_core::Node<'r, StrDoc<SupportLang>>),
) {
    let mut cursor = grep.root().get_inner_node().walk();
    loop {
        let node = cursor.node();
        let first = positions.partition_point(|&p| p < node.start_byte());
        if positions.get(first).is_some_and(|&p| p < node.end_byte()) {
            visit(grep.adopt(node));
            if cursor.goto_first_child() {
                continue;
            }
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return;
            }
        }
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
        let whole = AstGrepSet::with_tuning(
            AstGrepRule::from_yaml(yaml).unwrap(),
            Tuning {
                regions: false,
                ..Tuning::default()
            },
        );
        let regions = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
        let path = Path::new("m.py");
        let expected = whole.check(path, source);
        assert_eq!(expected.len(), 2);
        assert_eq!(regions.check(path, source), expected);
        assert_eq!(regions.stats().region_files, 1);
        assert!(regions.stats().parsed_bytes < source.len());
    }

    #[test]
    fn an_inside_that_reads_its_ancestor_keeps_the_whole_class() {
        // Narrowed to `g`, the class would lose `x = 1`, the `inside` would fail and the `not`
        // would report the print.
        let yaml = "id: p\nlanguage: python\nrule:\n  pattern: print($A)\n  not:\n    inside:\n      stopBy: end\n      pattern: |\n        class $C:\n            x = 1\n            $$$B\n";
        let filler: String = (0..40)
            .map(|i| format!("def h{i}(a, b):\n    return a + b * {i}\n\n"))
            .collect();
        let source = format!(
            "{filler}class C:\n    x = 1\n\n    def f(self):\n        return 1\n\n    def g(self):\n        print(2)\n"
        );
        let whole = AstGrepSet::with_tuning(
            AstGrepRule::from_yaml(yaml).unwrap(),
            Tuning {
                regions: false,
                ..Tuning::default()
            },
        );
        let regions = AstGrepSet::new(AstGrepRule::from_yaml(yaml).unwrap());
        let path = Path::new("m.py");
        assert!(whole.check(path, &source).is_empty());
        assert!(regions.check(path, &source).is_empty());
        assert_eq!(regions.stats().region_files, 1);
        assert!(AstGrepRule::from_yaml(yaml).unwrap()[0].inspects_descendants);
        // An ancestor that is never narrowed (a top-level `if`) does not stop the narrowing.
        let main = "id: p\nlanguage: python\nrule:\n  pattern: print($A)\n  not:\n    inside:\n      stopBy: end\n      pattern: |\n        if __name__ == \"__main__\":\n            $$$B\n";
        assert!(!AstGrepRule::from_yaml(main).unwrap()[0].inspects_descendants);
    }

    #[test]
    fn skipping_subtrees_finds_the_same_hits_as_matching_every_node() {
        let yaml = "id: p\nlanguage: python\nrule:\n  pattern: print($A)\n  not:\n    inside:\n      stopBy: end\n      pattern: 'if __name__ == \"__main__\": $$$B'\n---\nid: q\nlanguage: python\nrule:\n  kind: call\n  has:\n    field: function\n    regex: ^log$\n";
        let source = "def f(a):\n    return [x for x in a if print(x)]\n\nclass C:\n    def g(self):\n        log(print(2), print(3, 4))\n\nY = {'k': print(5)}\n\nif __name__ == \"__main__\":\n    print(6)\n";
        let every = AstGrepSet::with_tuning(
            AstGrepRule::from_yaml(yaml).unwrap(),
            Tuning {
                regions: false,
                skip_subtrees: false,
                ..Tuning::default()
            },
        );
        let skipping = AstGrepSet::with_tuning(
            AstGrepRule::from_yaml(yaml).unwrap(),
            Tuning {
                regions: false,
                ..Tuning::default()
            },
        );
        let path = Path::new("m.py");
        let expected = every.check(path, source);
        assert_eq!(expected.len(), 4);
        assert_eq!(skipping.check(path, source), expected);
        assert_eq!(skipping.stats().anchored_files, 1);
        assert_eq!(every.stats().anchored_files, 0);
    }

    #[test]
    fn covers_every_extension_ast_grep_maps_to_a_rule_language() {
        let set = AstGrepSet::new(
            AstGrepRule::from_yaml("id: j\nlanguage: javascript\nrule:\n  pattern: debugger\n")
                .unwrap(),
        );
        for linted in ["a.js", "a.mjs", "a.cjs", "a.jsx"] {
            assert!(set.covers(Path::new(linted)), "{linted}");
        }
        for other in ["a.py", "a.ts", "a.json"] {
            assert!(!set.covers(Path::new(other)), "{other}");
        }
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
