use super::{AstGrepSet, LanguageGroup};
use crate::chains;
use crate::language::{language_of, parse_reusing};
use crate::prefilter;
use crate::rule::{severity_name, Hit};
use ast_grep_core::matcher::MatcherExt;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::AstGrep;
use ast_grep_language::SupportLang;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::Ordering;

impl AstGrepSet {
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

    pub(super) fn run(
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
