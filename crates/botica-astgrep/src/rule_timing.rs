use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// What one ast-grep rule cost: the time spent trying it on nodes, summed over threads, and the
/// number of files it was tried on. The parse is shared by all rules and is not in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleTime {
    pub id: String,
    pub nanos: u64,
    pub files: usize,
}

/// One slot per rule. Only exists when asked for: the clock is read twice for every try of a rule
/// on a node.
pub(crate) struct RuleTimings {
    nanos: Vec<AtomicU64>,
    files: Vec<AtomicUsize>,
}

impl RuleTimings {
    pub(crate) fn new(rules: usize) -> Self {
        Self {
            nanos: (0..rules).map(|_| AtomicU64::new(0)).collect(),
            files: (0..rules).map(|_| AtomicUsize::new(0)).collect(),
        }
    }

    /// One matched file: how long each rule took on it, and which rules were tried.
    pub(crate) fn add(&self, spent: &[u64], active: &[bool]) {
        for (index, &nanos) in spent.iter().enumerate() {
            self.nanos[index].fetch_add(nanos, Ordering::Relaxed);
            if active[index] {
                self.files[index].fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub(crate) fn total(&self, index: usize) -> (u64, usize) {
        (
            self.nanos[index].load(Ordering::Relaxed),
            self.files[index].load(Ordering::Relaxed),
        )
    }
}
