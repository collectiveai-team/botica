//! Conservative "required literal" extraction from an ast-grep rule.
//!
//! A rule can only match a file whose text contains every identifier token of its `pattern`, so a
//! file that lacks them need not be parsed. The requirement is a boolean formula over literals.
//! Anything this cannot reason about (`kind`, `regex`, `matches`, `not`, patterns with a
//! non-default `strictness`) contributes no requirement, which only makes the filter weaker,
//! never wrong.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Req {
    Always,
    Lit(String),
    /// A regex (byte semantics, ASCII) the file text must match; declared by hand per rule.
    Re(String),
    And(Vec<Req>),
    Or(Vec<Req>),
}

impl Req {
    fn and(parts: Vec<Req>) -> Req {
        let mut parts: Vec<Req> = parts.into_iter().filter(|p| *p != Req::Always).collect();
        match parts.len() {
            0 => Req::Always,
            1 => parts.remove(0),
            _ => Req::And(parts),
        }
    }

    fn or(parts: Vec<Req>) -> Req {
        if parts.is_empty() || parts.contains(&Req::Always) {
            return Req::Always;
        }
        Req::Or(parts)
    }

    pub fn literals<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Req::Always => {}
            Req::Lit(literal) => out.push(literal),
            Req::Re(_) => {}
            Req::And(parts) | Req::Or(parts) => parts.iter().for_each(|p| p.literals(out)),
        }
    }

    pub fn regexes<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Req::Re(pattern) => out.push(pattern),
            Req::And(parts) | Req::Or(parts) => parts.iter().for_each(|p| p.regexes(out)),
            Req::Always | Req::Lit(_) => {}
        }
    }

    pub fn holds(&self, present: &mut dyn FnMut(&Req) -> bool) -> bool {
        match self {
            Req::Always => true,
            Req::Lit(_) | Req::Re(_) => present(self),
            Req::And(parts) => parts.iter().all(|p| p.holds(present)),
            Req::Or(parts) => parts.iter().any(|p| p.holds(present)),
        }
    }
}

pub fn requirement(rule: &Value) -> Req {
    let Some(object) = rule.as_object() else {
        return Req::Always;
    };
    let mut parts = Vec::new();
    for (key, value) in object {
        match key.as_str() {
            "pattern" => parts.push(pattern_requirement(value)),
            "all" => parts.push(Req::and(
                value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(requirement)
                    .collect(),
            )),
            "any" => parts.push(Req::or(
                value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(requirement)
                    .collect(),
            )),
            "inside" | "has" | "follows" | "precedes" => parts.push(requirement(value)),
            _ => {}
        }
    }
    Req::and(parts)
}

fn pattern_requirement(pattern: &Value) -> Req {
    let source = match pattern {
        Value::String(source) => source.as_str(),
        Value::Object(map) => {
            let relaxed = map
                .get("strictness")
                .and_then(Value::as_str)
                .is_some_and(|s| s != "smart" && s != "cst");
            match map.get("context").and_then(Value::as_str) {
                Some(context) if !relaxed => context,
                _ => return Req::Always,
            }
        }
        _ => return Req::Always,
    };
    Req::and(identifiers(source).into_iter().map(Req::Lit).collect())
}

/// Distinct identifier-like tokens of a pattern, skipping `$META` variables.
fn identifiers(source: &str) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut tokens: Vec<String> = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'$' {
            index += 1;
            while index < bytes.len() && (bytes[index] == b'$' || is_ident(bytes[index])) {
                index += 1;
            }
        } else if is_ident(byte) && !byte.is_ascii_digit() {
            let start = index;
            while index < bytes.len() && is_ident(bytes[index]) {
                index += 1;
            }
            let token = &source[start..index];
            if token.len() >= 2 && !tokens.iter().any(|t| t == token) {
                tokens.push(token.to_owned());
            }
        } else {
            index += 1;
        }
    }
    tokens
}

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn lit(text: &str) -> Req {
        Req::Lit(text.to_owned())
    }

    #[test]
    fn metavariables_are_not_literals() {
        assert_eq!(identifiers("print($$$ARGS)"), ["print"]);
        assert!(identifiers("$A = $B").is_empty());
        assert_eq!(
            identifiers("logging.getLogger($NAME)"),
            ["logging", "getLogger"]
        );
    }

    #[test]
    fn every_token_of_a_pattern_is_required() {
        let rule = json!({ "pattern": "return dict($$$ARGS)" });
        assert_eq!(
            requirement(&rule),
            Req::And(vec![lit("return"), lit("dict")])
        );
    }

    #[test]
    fn not_is_ignored_and_any_needs_every_branch() {
        let rule = json!({
            "pattern": "print($$$A)",
            "not": { "inside": { "pattern": "main_guard($B)" } },
        });
        assert_eq!(requirement(&rule), lit("print"));

        let weak = json!({ "any": [{ "pattern": "argparse.X($A)" }, { "kind": "call" }] });
        assert_eq!(requirement(&weak), Req::Always);

        let strong =
            json!({ "any": [{ "pattern": "import argparse" }, { "pattern": "import click" }] });
        assert_eq!(
            requirement(&strong),
            Req::Or(vec![
                Req::And(vec![lit("import"), lit("argparse")]),
                Req::And(vec![lit("import"), lit("click")]),
            ])
        );
    }
}
