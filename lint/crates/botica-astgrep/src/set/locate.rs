use super::{AstGrepSet, LanguageGroup};
use crate::chains;
use crate::language::parse_reusing;
use crate::prefilter;
use crate::regions;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::AstGrep;
use ast_grep_language::SupportLang;
use std::collections::HashMap;
use std::sync::atomic::Ordering;

/// Where the active rules of a file can match, from their prefilter atoms.
pub(super) struct Candidates {
    /// Sorted, without duplicates.
    pub(super) positions: Vec<usize>,
    /// No active rule uses `has`, so regions may narrow to class/function members.
    pub(super) members: bool,
    /// Every match of every active rule contains one of `positions`, so matching may skip
    /// the subtrees that hold none: no rule uses `inside` or a hand-written regex.
    pub(super) anchored: bool,
}

impl AstGrepSet {
    /// The candidate positions of the active rules, or `None` when some active rule cannot be
    /// located by them: it is not local, its prefilter is `Always`, or an atom is unknown.
    pub(super) fn candidates(
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
    pub(super) fn parse_regions(
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
}
