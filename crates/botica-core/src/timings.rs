use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedKind {
    Native,
    AstGrep,
    /// The reading of `botica:` and `noqa` comments, which is not a rule but costs like one.
    Suppressions,
}

impl TimedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::AstGrep => "syntax",
            Self::Suppressions => "suppressions",
        }
    }
}

/// What one rule cost: CPU time summed over the threads, and the files it looked at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleTime {
    pub rule: String,
    pub kind: TimedKind,
    pub nanos: u64,
    pub files: usize,
}

/// A slot per native rule and one more for the suppression comments. Only exists when asked for.
pub(crate) struct Timings {
    nanos: Vec<AtomicU64>,
    files: Vec<AtomicUsize>,
}

impl Timings {
    pub(crate) fn new(natives: usize) -> Self {
        let slots = natives + 1;
        Self {
            nanos: (0..slots).map(|_| AtomicU64::new(0)).collect(),
            files: (0..slots).map(|_| AtomicUsize::new(0)).collect(),
        }
    }

    pub(crate) fn suppressions_slot(&self) -> usize {
        self.nanos.len() - 1
    }

    pub(crate) fn measure<T>(&self, slot: usize, work: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let out = work();
        let nanos = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        self.nanos[slot].fetch_add(nanos, Ordering::Relaxed);
        self.files[slot].fetch_add(1, Ordering::Relaxed);
        out
    }

    pub(crate) fn total(&self, slot: usize) -> (u64, usize) {
        (
            self.nanos[slot].load(Ordering::Relaxed),
            self.files[slot].load(Ordering::Relaxed),
        )
    }
}
