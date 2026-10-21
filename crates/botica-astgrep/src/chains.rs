//! Automatic prefilter: what text a file must contain for an ast-grep rule to match, derived from
//! the rule itself, for any language with a tree-sitter grammar.
//!
//! A pattern is tokenized with the language's own grammar. Meta-variables split it into *chains*:
//! runs of tokens that must appear in the target in order. Between two tokens of a chain the target
//! may hold whitespace, comments, line continuations and *anonymous* tokens of the grammar
//! (keywords, punctuation), because ast-grep's default `smart` strictness skips unnamed target
//! nodes. Everything the analysis cannot reason about requires nothing, so the filter can only
//! get weaker, never wrong.
//!
//! All chains of a language are found in one pass: an Aho-Corasick automaton over their first
//! tokens, then a forward check of the remaining tokens at each candidate position.

use crate::prefilter::Req;
use aho_corasick::{AhoCorasick, MatchKind};
use ast_grep_language::{Language, LanguageExt, SupportLang};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Chain {
    tokens: Vec<Vec<u8>>,
    /// The token searched for: the first with a letter or digit, far rarer than punctuation.
    /// Tokens before it are checked backwards from it, tokens after it forwards.
    anchor: usize,
}

/// Per chain, whether it occurs; and every verified occurrence as `(chain, anchor offset)`.
pub type ChainScan = (Vec<bool>, Vec<(usize, usize)>);

pub struct LangChains {
    language: SupportLang,
    chains: Vec<Chain>,
    anchors: Option<AhoCorasick>,
    /// Anonymous tokens of the grammar, bucketed by first byte.
    anonymous_by_byte: Vec<Vec<Vec<u8>>>,
    anonymous_by_last: Vec<Vec<Vec<u8>>>,
    comments: Comments,
}

/// Comment syntax: line markers (to end of line) and an optional block pair.
#[derive(Clone, Copy)]
struct Comments {
    line: &'static [&'static [u8]],
    block: Option<(&'static [u8], &'static [u8])>,
}

const HASH: Comments = Comments {
    line: &[b"#"],
    block: None,
};
const DOUBLE_DASH: Comments = Comments {
    line: &[b"--"],
    block: None,
};
const C_FAMILY: Comments = Comments {
    line: &[b"//"],
    block: Some((b"/*", b"*/")),
};
const HTML_LIKE: Comments = Comments {
    line: &[],
    block: Some((b"<!--", b"-->")),
};
/// tree-sitter-javascript also lexes HTML-like comments (`<!--`, `-->`) as comments.
const JS_FAMILY: Comments = Comments {
    line: &[b"//", b"<!--", b"-->"],
    block: Some((b"/*", b"*/")),
};

impl LangChains {
    pub fn new(language: SupportLang) -> Self {
        let ts = language.get_ts_language();
        let mut anonymous: Vec<Vec<u8>> = (0..ts.node_kind_count() as u16)
            .filter(|&id| ts.node_kind_is_visible(id) && !ts.node_kind_is_named(id))
            .filter_map(|id| ts.node_kind_for_id(id))
            .filter(|kind| !kind.is_empty() && !kind.chars().any(char::is_whitespace))
            .map(|kind| kind.as_bytes().to_vec())
            .collect();
        anonymous.sort();
        anonymous.dedup();
        let mut anonymous_by_byte = vec![Vec::new(); 256];
        let mut anonymous_by_last = vec![Vec::new(); 256];
        for literal in anonymous {
            anonymous_by_last[literal[literal.len() - 1] as usize].push(literal.clone());
            anonymous_by_byte[literal[0] as usize].push(literal);
        }
        let comments = match language {
            SupportLang::Python
            | SupportLang::Ruby
            | SupportLang::Bash
            | SupportLang::Yaml
            | SupportLang::Elixir
            | SupportLang::Nix => HASH,
            SupportLang::Lua | SupportLang::Haskell => DOUBLE_DASH,
            SupportLang::JavaScript | SupportLang::TypeScript | SupportLang::Tsx => JS_FAMILY,
            SupportLang::Html | SupportLang::Markdown => HTML_LIKE,
            _ => C_FAMILY,
        };
        Self {
            language,
            chains: Vec::new(),
            anchors: None,
            anonymous_by_byte,
            anonymous_by_last,
            comments,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.chains.is_empty()
    }

    pub fn len(&self) -> usize {
        self.chains.len()
    }

    /// The requirement an ast-grep `rule:` object puts on the file text.
    pub fn requirement(&mut self, rule: &Value) -> Req {
        let Some(object) = rule.as_object() else {
            return Req::Always;
        };
        let mut parts = Vec::new();
        for (key, value) in object {
            let part = match key.as_str() {
                "pattern" => self.pattern(value),
                "regex" => value.as_str().map_or(Req::Always, regex_requirement),
                "all" => Req::and(self.each(value)),
                "any" => Req::or(self.each(value)),
                "inside" | "has" | "follows" | "precedes" => self.requirement(value),
                // `not`, `kind`, `matches`, `nthChild`, `range`, `stopBy`, `field`: no text needed.
                _ => Req::Always,
            };
            parts.push(part);
        }
        Req::and(parts)
    }

    fn each(&mut self, value: &Value) -> Vec<Req> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .map(|v| self.requirement(v))
            .collect()
    }

    fn pattern(&mut self, pattern: &Value) -> Req {
        let source = match pattern {
            Value::String(source) => source.as_str(),
            Value::Object(map) => {
                let strictness = map.get("strictness").and_then(Value::as_str);
                // `selector` keeps only part of `context`; relaxed strictness skips named nodes too.
                if map.contains_key("selector")
                    || strictness.is_some_and(|s| s != "smart" && s != "cst")
                {
                    return Req::Always;
                }
                match map.get("context").and_then(Value::as_str) {
                    Some(context) => context,
                    None => return Req::Always,
                }
            }
            _ => return Req::Always,
        };
        let chains = self.tokenize(source);
        Req::and(chains.into_iter().map(|chain| self.intern(chain)).collect())
    }

    fn tokenize(&self, source: &str) -> Vec<Chain> {
        let processed = self.language.pre_process_pattern(source);
        let grep = self.language.ast_grep(processed.as_ref());
        let root = grep.root();
        if root.get_inner_node().has_error() {
            return Vec::new();
        }
        let expando = self.language.expando_char();
        let mut chains = Vec::new();
        let mut current: Vec<Vec<u8>> = Vec::new();
        // Byte range of the previous token's parent. In `smart` mode ast-grep accepts any
        // trailing children once a node's pattern children are matched, so a token outside
        // that parent need not follow the previous one directly: the chain must break there.
        let mut parent: Option<std::ops::Range<usize>> = None;
        for node in root.dfs() {
            if node.get_inner_node().child_count() > 0 {
                continue;
            }
            let text = node.text();
            if text.contains(expando) || text.contains('$') {
                chains.push(std::mem::take(&mut current));
                parent = None;
                continue;
            }
            if text.is_empty() || node.kind().contains("comment") {
                continue;
            }
            let range = node.range();
            if parent
                .as_ref()
                .is_some_and(|p| range.start < p.start || range.end > p.end)
            {
                chains.push(std::mem::take(&mut current));
            }
            parent = node.parent().map(|p| p.range());
            current.push(text.as_bytes().to_vec());
        }
        chains.push(current);
        chains
            .into_iter()
            .filter(|tokens| {
                tokens
                    .iter()
                    .any(|t| t.iter().any(u8::is_ascii_alphanumeric))
            })
            .map(|tokens| {
                let anchor = tokens
                    .iter()
                    .position(|t| t.iter().any(u8::is_ascii_alphanumeric))
                    .unwrap_or(0);
                Chain { tokens, anchor }
            })
            .collect()
    }

    fn intern(&mut self, chain: Chain) -> Req {
        let index = match self.chains.iter().position(|c| *c == chain) {
            Some(index) => index,
            None => {
                self.chains.push(chain);
                self.chains.len() - 1
            }
        };
        Req::Chain(index)
    }

    /// Builds the automaton once every rule has been interned.
    pub fn finish(&mut self) {
        let firsts: Vec<&[u8]> = self
            .chains
            .iter()
            .map(|c| c.tokens[c.anchor].as_slice())
            .collect();
        self.anchors = (!firsts.is_empty()).then(|| {
            AhoCorasick::builder()
                .match_kind(MatchKind::Standard)
                .build(firsts)
                .expect("chain anchors are plain literals")
        });
    }

    /// Which chains occur in `text`, as a flag per chain.
    #[cfg(test)]
    pub fn scan(&self, text: &[u8]) -> Vec<bool> {
        self.scan_all(text).0
    }

    /// Which chains occur in `text`, as a flag per chain, and every verified occurrence as
    /// `(chain, anchor offset)`, for region parsing.
    pub fn scan_all(&self, text: &[u8]) -> ChainScan {
        let mut found = vec![false; self.chains.len()];
        let mut at = Vec::new();
        let Some(anchors) = &self.anchors else {
            return (found, at);
        };
        for hit in anchors.find_overlapping_iter(text) {
            let index = hit.pattern().as_usize();
            if !bounded(text, hit.start(), hit.end()) {
                continue;
            }
            let chain = &self.chains[index];
            if self.rest_follows(&chain.tokens[chain.anchor + 1..], text, hit.end())
                && self.rest_precedes(&chain.tokens[..chain.anchor], text, hit.start())
            {
                found[index] = true;
                at.push((index, hit.start()));
            }
        }
        (found, at)
    }

    fn rest_precedes(&self, tokens: &[Vec<u8>], text: &[u8], mut position: usize) -> bool {
        for token in tokens.iter().rev() {
            match self.before_gap(text, position, token) {
                Some(start) => position = start,
                None => return false,
            }
        }
        true
    }

    /// Mirror of `after_gap`, walking backwards. It skips comments too: ast-grep 0.41-0.44 stop
    /// a `smart` match at a comment, but 0.45 skips comments (`is_extra`), so the prefilter must
    /// allow them to stay correct across versions.
    fn before_gap(&self, text: &[u8], end: usize, token: &[u8]) -> Option<usize> {
        let mut pending = vec![end];
        let mut seen: Vec<usize> = Vec::new();
        while let Some(at) = pending.pop() {
            if seen.contains(&at) {
                continue;
            }
            seen.push(at);
            let head = &text[..at];
            if head.ends_with(token) && bounded(text, at - token.len(), at) {
                return Some(at - token.len());
            }
            self.skips_ending_at(text, at, &mut pending);
        }
        None
    }

    /// Pushes onto `pending` the start of every single skip that ends at `at`.
    fn skips_ending_at(&self, text: &[u8], at: usize, pending: &mut Vec<usize>) {
        let head = &text[..at];
        let mut back = |len: usize| {
            if len > 0 {
                pending.push(at - len);
            }
        };
        back(
            head.iter()
                .rev()
                .take_while(|b| b.is_ascii_whitespace())
                .count(),
        );
        back(continuation_len_before(head));
        let literals = head.last().map_or(&[][..], |&last| {
            self.anonymous_by_last[last as usize].as_slice()
        });
        for literal in literals.iter().filter(|literal| head.ends_with(literal)) {
            back(literal.len());
        }
        let mut starts = Vec::new();
        let at_line_end = text.get(at).is_none_or(|&b| b == b'\n' || b == b'\r');
        self.comment_starts(head, at_line_end, &mut starts);
        pending.extend(starts.into_iter().filter(|&start| start < at));
    }

    fn rest_follows(&self, tokens: &[Vec<u8>], text: &[u8], mut position: usize) -> bool {
        for token in tokens {
            match self.after_gap(text, position, token) {
                Some(end) => position = end,
                None => return false,
            }
        }
        true
    }

    /// Finds `token` after a gap the matcher would skip; explores every way to split the gap.
    fn after_gap(&self, text: &[u8], start: usize, token: &[u8]) -> Option<usize> {
        // Fast path: almost every gap is plain whitespace or nothing.
        let at = start
            + text[start..]
                .iter()
                .take_while(|b| b.is_ascii_whitespace())
                .count();
        if text[at..].starts_with(token) && bounded(text, at, at + token.len()) {
            return Some(at + token.len());
        }
        let next_byte = text.get(at).copied();
        let may_skip = next_byte.is_some_and(|b| {
            b == b'\\'
                || !self.anonymous_by_byte[b as usize].is_empty()
                || self.comment_len(&text[at..]) > 0
        });
        if !may_skip {
            return None;
        }
        self.after_gap_slow(text, start, token)
    }

    fn after_gap_slow(&self, text: &[u8], start: usize, token: &[u8]) -> Option<usize> {
        let mut pending = vec![start];
        let mut seen: Vec<usize> = Vec::new();
        while let Some(at) = pending.pop() {
            if seen.contains(&at) {
                continue;
            }
            seen.push(at);
            let rest = &text[at..];
            if rest.starts_with(token) && bounded(text, at, at + token.len()) {
                return Some(at + token.len());
            }
            self.skips_starting_at(text, at, &mut pending);
        }
        None
    }

    /// Pushes onto `pending` the end of every single skip that starts at `at`.
    fn skips_starting_at(&self, text: &[u8], at: usize, pending: &mut Vec<usize>) {
        let rest = &text[at..];
        let mut next = |len: usize| {
            if len > 0 {
                pending.push(at + len);
            }
        };
        next(rest.iter().take_while(|b| b.is_ascii_whitespace()).count());
        next(continuation_len_after(rest));
        next(self.comment_len(rest));
        let literals = rest.first().map_or(&[][..], |&first| {
            self.anonymous_by_byte[first as usize].as_slice()
        });
        for literal in literals.iter().filter(|literal| rest.starts_with(literal)) {
            next(literal.len());
        }
    }

    fn comment_len(&self, rest: &[u8]) -> usize {
        if let Some((open, close)) = self.comments.block {
            if rest.starts_with(open) {
                return memchr::memmem::find(&rest[open.len()..], close)
                    .map_or(rest.len(), |end| open.len() + end + close.len());
            }
        }
        if self
            .comments
            .line
            .iter()
            .any(|marker| rest.starts_with(marker))
        {
            return rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
        }
        0
    }

    /// Start positions of every comment that could end exactly at `head.len()`. A line comment
    /// may start at any marker of the current line (a marker inside a string is a harmless extra
    /// candidate); a block comment at any opener after the previous closer.
    fn comment_starts(&self, head: &[u8], at_line_end: bool, out: &mut Vec<usize>) {
        if at_line_end {
            let line_start = head.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
            let line = &head[line_start..];
            for marker in self.comments.line {
                for found in memchr::memmem::find_iter(line, marker) {
                    out.push(line_start + found);
                }
            }
        }
        if let Some((open, close)) = self.comments.block {
            if head.ends_with(close) {
                let body = &head[..head.len() - close.len()];
                let from = memchr::memmem::rfind(body, close).map_or(0, |i| i + close.len());
                for found in memchr::memmem::find_iter(&body[from..], open) {
                    out.push(from + found);
                }
            }
        }
    }
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}

/// The length of a line continuation (`\` and a newline) that `head` ends with, or 0.
fn continuation_len_before(head: &[u8]) -> usize {
    if head.ends_with(b"\\\r\n") {
        3
    } else if head.ends_with(b"\\\n") {
        2
    } else {
        0
    }
}

/// The length of a line continuation (`\` and a newline) that `rest` starts with, or 0.
fn continuation_len_after(rest: &[u8]) -> usize {
    if rest.starts_with(b"\\\r\n") {
        3
    } else if rest.starts_with(b"\\\n") {
        2
    } else {
        0
    }
}

/// A token that starts or ends with a word character must not continue a longer word.
fn bounded(text: &[u8], start: usize, end: usize) -> bool {
    let clean_start = !is_word(text[start]) || start == 0 || !is_word(text[start - 1]);
    let clean_end = !is_word(text[end - 1]) || end == text.len() || !is_word(text[end]);
    clean_start && clean_end
}

/// Every match of the regex starts with one of its prefix literals, so the file contains one.
fn regex_requirement(pattern: &str) -> Req {
    use regex_syntax::hir::literal::{ExtractKind, Extractor};
    let Ok(hir) = regex_syntax::parse(pattern) else {
        return Req::Always;
    };
    let mut extractor = Extractor::new();
    extractor.kind(ExtractKind::Prefix);
    let seq = extractor.extract(&hir);
    match seq.literals() {
        Some(literals) if !literals.is_empty() && literals.iter().all(|l| !l.is_empty()) => {
            Req::or(
                literals
                    .iter()
                    .map(|l| Req::Lit(String::from_utf8_lossy(l.as_bytes()).into_owned()))
                    .collect(),
            )
        }
        _ => Req::Always,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn chains_of(lang: SupportLang, rule: Value) -> (LangChains, Req) {
        let mut chains = LangChains::new(lang);
        let req = chains.requirement(&rule);
        chains.finish();
        (chains, req)
    }

    fn matches(chains: &LangChains, req: &Req, text: &str) -> bool {
        let found = chains.scan(text.as_bytes());
        req.holds(&mut |atom| match atom {
            Req::Chain(i) => found[*i],
            Req::Lit(l) => text.contains(l.as_str()),
            _ => true,
        })
    }

    #[test]
    fn metavariables_split_patterns_into_chains() {
        let (chains, _) = chains_of(
            SupportLang::Python,
            json!({"pattern": "return dict($$$ARGS)"}),
        );
        let tokens: Vec<Vec<String>> = chains
            .chains
            .iter()
            .map(|c| {
                c.tokens
                    .iter()
                    .map(|t| String::from_utf8_lossy(t).into_owned())
                    .collect()
            })
            .collect();
        assert_eq!(tokens, vec![vec!["return", "dict", "("]]);
    }

    #[test]
    fn gaps_allow_trivia_and_anonymous_tokens_but_not_names() {
        let (c, r) = chains_of(
            SupportLang::Python,
            json!({"pattern": "return dict($$$ARGS)"}),
        );
        assert!(matches(&c, &r, "def f():\n    return dict(a=1)\n"));
        assert!(matches(
            &c,
            &r,
            "def f():\n    return \\\n        dict (a=1)\n"
        ));
        assert!(!matches(
            &c,
            &r,
            "def f():\n    x = dict(a=1)\n    return x\n"
        ));
        assert!(!matches(&c, &r, "return dictionary(a)"));
        assert!(!matches(&c, &r, "return my_dict(a)"));
    }

    #[test]
    fn python_comments_are_hash_only() {
        let (c, r) = chains_of(SupportLang::Python, json!({"pattern": "$A = dict"}));
        assert!(matches(&c, &r, "x = dict\n"));
        assert!(matches(&c, &r, "x: int = dict\n"));
        assert!(!matches(&c, &r, "x = dictionary\n"));
        // `//` is floor division in Python: treating it as a comment would skip `dict` here.
        let (c, r) = chains_of(SupportLang::Python, json!({"pattern": "$A // dict"}));
        assert!(matches(&c, &r, "y = a // dict\n"));
    }

    #[test]
    fn not_and_kind_require_nothing_and_any_needs_a_branch() {
        let (_, r) = chains_of(
            SupportLang::Python,
            json!({"kind": "dictionary", "not": {"pattern": "x"}}),
        );
        assert_eq!(r, Req::Always);
        let (c, r) = chains_of(
            SupportLang::Python,
            json!({"any": [{"pattern": "import argparse"}, {"pattern": "sys.argv"}]}),
        );
        assert!(matches(&c, &r, "import sys\nargs = sys.argv\n"));
        assert!(matches(&c, &r, "import  argparse\n"));
        assert!(!matches(&c, &r, "import sys\nprint(argv)\n"));
    }

    #[test]
    fn punctuation_before_the_anchor_is_checked_backwards() {
        let (c, r) = chains_of(
            SupportLang::Python,
            json!({"pattern": "def $F($$$A) -> dict:\n    $$$B"}),
        );
        assert!(matches(&c, &r, "def f(a) -> dict:\n    pass\n"));
        assert!(matches(&c, &r, "def f(a)->dict:\n    pass\n"));
        assert!(matches(&c, &r, "def f(a) -> \\\n        dict:\n    pass\n"));
        assert!(!matches(&c, &r, "def f(a) -> int:\n    x: dict = {}\n"));
        let (c, r) = chains_of(
            SupportLang::Python,
            json!({"pattern": "class $N(SQLModel, table=True):\n    $$$B"}),
        );
        assert!(matches(
            &c,
            &r,
            "class Row(SQLModel, table=True):\n    id: int\n"
        ));
        assert!(matches(
            &c,
            &r,
            "class Row (  SQLModel ,table = True ):\n    id: int\n"
        ));
        assert!(!matches(
            &c,
            &r,
            "class Row(Base, SQLModel):\n    table = True\n"
        ));
    }

    #[test]
    fn comments_inside_a_chain_are_allowed_both_ways() {
        // ast-grep 0.45 skips comments in `smart` mode; the check before the anchor must too.
        let (c, r) = chains_of(
            SupportLang::Python,
            json!({"pattern": "class $N(SQLModel, table=True):\n    $$$B"}),
        );
        assert!(matches(
            &c,
            &r,
            "class Hero(  # db model\n    SQLModel, table=True):\n    id: int\n"
        ));
        assert!(matches(
            &c,
            &r,
            "class Hero(  # a # b\n    SQLModel, table=True):\n    id: int\n"
        ));
        let (c, r) = chains_of(
            SupportLang::TypeScript,
            json!({"pattern": "let $A: any = $B"}),
        );
        assert!(matches(&c, &r, "let a: /* c */ any = 1;"));
        assert!(matches(&c, &r, "let a: // c\n  any = 1;"));
        let (c, r) = chains_of(
            SupportLang::JavaScript,
            json!({"pattern": "throw new Error($$$A)"}),
        );
        assert!(matches(&c, &r, "throw <!-- c\n new Error(1);"));
    }

    #[test]
    fn chains_break_where_ast_grep_accepts_trailing_children() {
        // ast-grep matches `from click import $$$X` on `from click.shell_completion import X`:
        // the module's dotted name may carry extra trailing parts.
        let (c, r) = chains_of(
            SupportLang::Python,
            json!({"pattern": "from click import $$$X"}),
        );
        assert!(matches(
            &c,
            &r,
            "from click.shell_completion import CompletionItem\n"
        ));
        assert!(matches(&c, &r, "from click import echo\n"));
        assert!(!matches(&c, &r, "from clicker import echo\n"));
    }

    #[test]
    fn regex_contributes_its_required_literals() {
        assert_eq!(regex_requirement("BaseModel"), Req::Lit("BaseModel".into()));
        assert_eq!(regex_requirement(".*"), Req::Always);
        assert!(matches!(regex_requirement("foo|bar"), Req::Or(_)));
    }

    #[test]
    fn typescript_uses_c_family_comments() {
        let (c, r) = chains_of(
            SupportLang::TypeScript,
            json!({"pattern": "console.log($$$A)"}),
        );
        assert!(matches(&c, &r, "console . /* x */ log(1);"));
        assert!(!matches(&c, &r, "console.error(1);"));
    }
}
