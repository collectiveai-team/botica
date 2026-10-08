//! What a file's text must contain for a rule to match: a boolean formula over cheap text checks.
//!
//! The formula is derived from each rule by `chains`, or declared by hand for rules without a
//! pattern (`kind:` only). `Always` means "nothing is known", so a weaker formula is never wrong.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Req {
    Always,
    /// A literal the text must contain (from a `regex:` rule's mandatory prefixes).
    Lit(String),
    /// A regex (byte semantics, ASCII) the text must match; declared by hand per rule.
    Re(String),
    /// A token chain of `chains::LangChains`, by index.
    Chain(usize),
    And(Vec<Req>),
    Or(Vec<Req>),
}

impl Req {
    pub fn and(parts: Vec<Req>) -> Req {
        let mut parts: Vec<Req> = parts.into_iter().filter(|p| *p != Req::Always).collect();
        match parts.len() {
            0 => Req::Always,
            1 => parts.remove(0),
            _ => Req::And(parts),
        }
    }

    pub fn or(mut parts: Vec<Req>) -> Req {
        if parts.is_empty() || parts.contains(&Req::Always) {
            return Req::Always;
        }
        if parts.len() == 1 {
            return parts.remove(0);
        }
        Req::Or(parts)
    }

    pub fn literals<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Req::Lit(literal) => out.push(literal),
            Req::And(parts) | Req::Or(parts) => parts.iter().for_each(|p| p.literals(out)),
            Req::Always | Req::Re(_) | Req::Chain(_) => {}
        }
    }

    pub fn regexes<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Req::Re(pattern) => out.push(pattern),
            Req::And(parts) | Req::Or(parts) => parts.iter().for_each(|p| p.regexes(out)),
            Req::Always | Req::Lit(_) | Req::Chain(_) => {}
        }
    }

    /// Visits every text check of the formula, in every branch.
    pub fn each_atom(&self, visit: &mut dyn FnMut(&Req)) {
        match self {
            Req::And(parts) | Req::Or(parts) => parts.iter().for_each(|p| p.each_atom(visit)),
            Req::Always => {}
            atom => visit(atom),
        }
    }

    /// Collects every branch of an `Or` but, of an `And`, only the part with the fewest
    /// occurrences by `count`: any part locates a match when every part lies inside the matched
    /// node. Returns the occurrences collected.
    pub fn rarest_atoms<'a>(
        &'a self,
        count: &mut dyn FnMut(&Req) -> usize,
        out: &mut Vec<&'a Req>,
    ) -> usize {
        match self {
            Req::Or(parts) => parts.iter().map(|p| p.rarest_atoms(count, out)).sum(),
            Req::And(parts) => {
                let mut best: Option<(usize, Vec<&'a Req>)> = None;
                for part in parts {
                    let mut atoms = Vec::new();
                    let total = part.rarest_atoms(count, &mut atoms);
                    if best.as_ref().is_none_or(|(least, _)| total < *least) {
                        best = Some((total, atoms));
                    }
                }
                let (total, atoms) = best.unwrap_or_default();
                out.extend(atoms);
                total
            }
            Req::Always => 0,
            atom => {
                out.push(atom);
                count(atom)
            }
        }
    }

    pub fn holds(&self, present: &mut dyn FnMut(&Req) -> bool) -> bool {
        match self {
            Req::Always => true,
            Req::Lit(_) | Req::Re(_) | Req::Chain(_) => present(self),
            Req::And(parts) => parts.iter().all(|p| p.holds(present)),
            Req::Or(parts) => parts.iter().any(|p| p.holds(present)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_is_neutral_for_and_and_absorbing_for_or() {
        let lit = Req::Lit("x".into());
        assert_eq!(Req::and(vec![Req::Always, lit.clone()]), lit);
        assert_eq!(Req::and(vec![Req::Always]), Req::Always);
        assert_eq!(Req::or(vec![Req::Always, lit.clone()]), Req::Always);
        assert_eq!(Req::or(vec![lit.clone()]), lit);
    }

    #[test]
    fn rarest_atoms_keep_every_or_branch_and_the_rarest_and_part() {
        // `def` is everywhere; `-> dict` is rare: an `And` of both is located by the latter.
        let req = Req::Or(vec![
            Req::And(vec![Req::Chain(0), Req::Chain(1)]),
            Req::And(vec![
                Req::Chain(0),
                Req::Or(vec![Req::Chain(2), Req::Chain(3)]),
            ]),
        ]);
        let occurrences = [50, 1, 2, 60];
        let mut count = |atom: &Req| match atom {
            Req::Chain(i) => occurrences[*i],
            _ => 0,
        };
        let mut atoms = Vec::new();
        let total = req.rarest_atoms(&mut count, &mut atoms);
        assert_eq!(atoms, [&Req::Chain(1), &Req::Chain(0)]);
        assert_eq!(total, 51);
    }
}
