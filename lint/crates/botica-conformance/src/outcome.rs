use std::fmt;

use crate::finding::Finding;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// `fixtures/valid` or `fixtures/invalid` is missing or holds no file.
    NoFiles { side: String },
    /// There are invalid fixtures but no `expected.json` to say what they must raise.
    NoExpected,
    /// `expected.json` lists nothing, so the invalid fixtures would be valid.
    ExpectsNothing,
    /// A `valid` fixture raised a diagnostic of the rule.
    Unexpected { variant: String, finding: Finding },
    /// The `invalid` fixtures raised something other than what `expected.json` lists.
    Mismatch {
        variant: String,
        expected: Vec<Finding>,
        actual: Vec<Finding>,
    },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFiles { side } => write!(f, "fixtures/{side} has no files"),
            Self::NoExpected => write!(f, "fixtures/expected.json is missing"),
            Self::ExpectsNothing => write!(
                f,
                "fixtures/expected.json lists nothing: the invalid fixtures must raise something"
            ),
            Self::Unexpected { variant, finding } => {
                write!(f, "[{variant}] a valid fixture raised: {finding}")
            }
            Self::Mismatch {
                variant,
                expected,
                actual,
            } => {
                writeln!(
                    f,
                    "[{variant}] the invalid fixtures do not give what expected.json lists"
                )?;
                write_list(f, "expected", expected)?;
                write_list(f, "actual", actual)
            }
        }
    }
}

fn write_list(f: &mut fmt::Formatter<'_>, label: &str, findings: &[Finding]) -> fmt::Result {
    writeln!(f, "  {label}:")?;
    if findings.is_empty() {
        return writeln!(f, "    (nothing)");
    }
    for finding in findings {
        writeln!(f, "    {finding}")?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    rule: String,
    failures: Vec<Failure>,
}

impl Outcome {
    pub(crate) fn new(rule: &str, failures: Vec<Failure>) -> Self {
        Self {
            rule: rule.to_owned(),
            failures,
        }
    }

    pub fn rule(&self) -> &str {
        &self.rule
    }

    pub fn failures(&self) -> &[Failure] {
        &self.failures
    }

    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}
