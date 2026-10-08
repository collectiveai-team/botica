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

    /// Visits every branch of an `Or` but only the first part of an `And`: enough to locate a
    /// match when every part of an `And` lies inside the matched node.
    pub fn each_first_atom(&self, visit: &mut dyn FnMut(&Req)) {
        match self {
            Req::Or(parts) => parts.iter().for_each(|p| p.each_first_atom(visit)),
            Req::And(parts) => {
                if let Some(first) = parts.first() {
                    first.each_first_atom(visit);
                }
            }
            Req::Always => {}
            atom => visit(atom),
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
}
