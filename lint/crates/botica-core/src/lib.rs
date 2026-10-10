mod diagnostic;
mod document;
mod engine;
mod native;
pub mod postprocess;
pub mod registry;
pub mod report;
mod shape;
pub mod suppress;
mod timings;

#[cfg(feature = "fs")]
pub mod cache;
#[cfg(feature = "fs")]
pub mod walk;

pub use diagnostic::{Diagnostic, Severity};
pub use document::Document;
pub use engine::{language_of, Engine, Exclusion, ExtensionRun, NativePlanner, Plan};
pub use native::{FileSizeGuard, NativeRule, Needs, NoUtils};
pub use shape::RepoShape;
pub use timings::{RuleTime, TimedKind};
