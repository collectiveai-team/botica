use std::fmt;

/// The two folders of a rule's fixtures: code that must stay quiet and code that must be flagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Valid,
    Invalid,
}

impl Side {
    pub fn folder(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::Invalid => "invalid",
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.folder())
    }
}
