//! The Python versions that a rule asks of a project (`python` of `rule.toml`), and the lowest
//! version that the `requires-python` of a project allows (ADR 0015).

use std::cmp::Ordering;
use std::fmt;

use serde::Deserialize;

/// A release of Python, `3.15` or `3.14.2`; a missing part counts as 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PythonVersion {
    parts: [u64; 3],
    /// How many parts the text wrote, so `3.15` prints as `3.15`.
    written: usize,
}

impl PythonVersion {
    pub const fn new(major: u64, minor: u64) -> Self {
        Self {
            parts: [major, minor, 0],
            written: 2,
        }
    }

    /// `3`, `3.15` or `3.15.1`, with nothing else.
    fn parse(text: &str) -> Option<Self> {
        let mut parts = [0; 3];
        let mut written = 0;
        for part in text.split('.') {
            if written == 3 || part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            parts[written] = part.parse().ok()?;
            written += 1;
        }
        (written > 0).then_some(Self { parts, written })
    }

    /// The release that a version of a specifier of PEP 440 starts with: `3.15` of `3.15.0b1`,
    /// `3.15.*` or `3.15rc1`. An epoch or a local part is not read.
    fn release_of(text: &str) -> Option<Self> {
        let end = text
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(text.len());
        Self::parse(text[..end].trim_end_matches('.'))
    }

    /// The first release after this one: `3.14` gives `3.14.1`, as `>3.14` allows `3.14.1`
    /// (PEP 440).
    fn next(self) -> Self {
        Self {
            parts: [self.parts[0], self.parts[1], self.parts[2] + 1],
            written: 3,
        }
    }
}

impl Ord for PythonVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parts.cmp(&other.parts)
    }
}

impl PartialOrd for PythonVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.parts[..self.written]
            .iter()
            .map(u64::to_string)
            .collect();
        f.write_str(&parts.join("."))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    AtLeast,
    Above,
    AtMost,
    Below,
    Equal,
    NotEqual,
}

impl Operator {
    /// The operator at the start of `text` and the rest; the longest operator first, so `>=` is
    /// not read as `>`.
    fn split(text: &str) -> Option<(Self, &str)> {
        [
            (">=", Self::AtLeast),
            ("<=", Self::AtMost),
            ("==", Self::Equal),
            ("!=", Self::NotEqual),
            (">", Self::Above),
            ("<", Self::Below),
        ]
        .into_iter()
        .find_map(|(symbol, operator)| {
            text.strip_prefix(symbol)
                .map(|rest| (operator, rest.trim()))
        })
    }

    fn holds(self, version: PythonVersion, bound: PythonVersion) -> bool {
        let order = version.cmp(&bound);
        match self {
            Self::AtLeast => order != Ordering::Less,
            Self::Above => order == Ordering::Greater,
            Self::AtMost => order != Ordering::Greater,
            Self::Below => order == Ordering::Less,
            Self::Equal => order == Ordering::Equal,
            Self::NotEqual => order != Ordering::Equal,
        }
    }
}

/// The `python` of a rule: clauses joined by commas that each hold, `>=3.15` or `>=3.10, <3.15`.
/// Each clause is one of `>=`, `>`, `<=`, `<`, `==` and `!=` and a release.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct PythonRange {
    text: String,
    clauses: Vec<(Operator, PythonVersion)>,
}

impl PythonRange {
    /// Whether `version` is inside the range.
    pub fn contains(&self, version: PythonVersion) -> bool {
        self.clauses
            .iter()
            .all(|(operator, bound)| operator.holds(version, *bound))
    }
}

impl TryFrom<String> for PythonRange {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let clauses = text
            .split(',')
            .map(|clause| {
                let (operator, version) = Operator::split(clause.trim())?;
                Some((operator, PythonVersion::parse(version)?))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                format!(
                    "`python` is a range of Python versions such as `>=3.15` or `>=3.10, <3.15`, \
                     not `{text}`"
                )
            })?;
        Ok(Self { text, clauses })
    }
}

impl fmt::Display for PythonRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

/// The lowest release that a `requires-python` allows, from its clauses `>=`, `>`, `==`, `===`
/// and `~=`: `>=3.12, <4` gives `3.12`, `~=3.15.0` gives `3.15.0`, `>3.14` gives `3.14.1`. `None`
/// when no clause sets a lower bound (`<4`) or a clause cannot be read.
pub fn lower_bound(requires_python: &str) -> Option<PythonVersion> {
    let mut lowest: Option<PythonVersion> = None;
    for clause in requires_python.split(',').map(str::trim) {
        let bound = if let Some(version) = clause
            .strip_prefix("===")
            .or_else(|| clause.strip_prefix("=="))
            .or_else(|| clause.strip_prefix("~="))
            .or_else(|| clause.strip_prefix(">="))
        {
            Some(PythonVersion::release_of(version.trim())?)
        } else if let Some(version) = clause.strip_prefix('>') {
            Some(PythonVersion::release_of(version.trim())?.next())
        } else if ["<", "!="].iter().any(|symbol| clause.starts_with(symbol)) {
            None
        } else {
            return None;
        };
        lowest = lowest.max(bound);
    }
    lowest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(text: &str) -> PythonRange {
        PythonRange::try_from(text.to_owned()).expect("the range parses")
    }

    fn version(text: &str) -> PythonVersion {
        PythonVersion::parse(text).expect("the version parses")
    }

    #[test]
    fn a_range_holds_the_versions_that_every_clause_holds() {
        assert!(range(">=3.15").contains(version("3.15")));
        assert!(range(">=3.15").contains(version("3.15.0")));
        assert!(!range(">=3.15").contains(version("3.14.9")));
        assert!(range("<3.15").contains(version("3.14")));
        assert!(!range("<3.15").contains(version("3.15")));
        assert!(range(">=3.10, <3.15").contains(version("3.12")));
        assert!(!range(">=3.10, <3.15").contains(version("3.9")));
        assert!(range(">3.14").contains(version("3.14.1")));
        assert!(range("!=3.13").contains(version("3.14")));
        assert!(range("==3.14").contains(version("3.14.0")));
        assert!(range("<=3.14").contains(version("3.14")));
    }

    #[test]
    fn a_range_that_is_not_a_list_of_clauses_is_refused() {
        for text in [
            "",
            "3.15",
            ">=",
            ">=3.x",
            "~=3.15",
            ">=3.15.*",
            ">=3.15.0.1",
            ">=3.15,",
        ] {
            assert!(PythonRange::try_from(text.to_owned()).is_err(), "{text}");
        }
    }

    #[test]
    fn the_range_prints_as_written() {
        assert_eq!(range(">=3.10, <3.15").to_string(), ">=3.10, <3.15");
    }

    #[test]
    fn the_lower_bound_is_the_highest_of_the_clauses_that_set_one() {
        let cases = [
            (">=3.12", "3.12"),
            (">=3.12,<4", "3.12"),
            (" >= 3.15 , != 3.15.1", "3.15"),
            ("~=3.15.0", "3.15.0"),
            ("==3.15.*", "3.15"),
            ("===3.14.2", "3.14.2"),
            (">3.14", "3.14.1"),
            (">3.14.1", "3.14.2"),
            (">=3.15.0b1", "3.15.0"),
            (">=3.10,>=3.12", "3.12"),
        ];
        for (requires, lowest) in cases {
            assert_eq!(
                lower_bound(requires).map(|v| v.to_string()).as_deref(),
                Some(lowest),
                "{requires}"
            );
        }
    }

    #[test]
    fn a_requires_python_with_no_lower_bound_or_unreadable_has_none() {
        for requires in ["<4", "!=3.13", "", "3.12", ">=three"] {
            assert_eq!(lower_bound(requires), None, "{requires}");
        }
    }
}
