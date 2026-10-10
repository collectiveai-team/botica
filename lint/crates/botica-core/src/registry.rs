use std::collections::BTreeMap;
use std::fmt;

use botica_catalog::{Catalog, Kind};

use crate::layout::{CodeDir, NoArtifactsInPackage, NoNotebooks, NoTestsInPackage};
use crate::native::{FileSizeGuard, NativeRule, NoUtils};
use crate::suppress::{INVALID, UNUSED};

pub const IO_ERROR: &str = "io-error";

/// The effective options of a rule: its catalog defaults with the configuration on top.
pub type Options = BTreeMap<String, toml::Value>;

/// What a rule may need to know about the project that is not one of its options.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    /// Where the code of the project lives, as paths in the terms of the paths the rules get.
    pub code_dirs: Vec<CodeDir>,
}

enum Provider {
    Rule(fn(&Options, &Context) -> Box<dyn NativeRule>),
    /// The engine reports these itself while it lints (suppression comments, unreadable files).
    Engine,
}

const NATIVES: [(&str, Provider); 8] = [
    ("file-size-guard", Provider::Rule(file_size_guard)),
    ("no-utils", Provider::Rule(no_utils)),
    ("no-tests-in-package", Provider::Rule(no_tests_in_package)),
    ("no-notebooks", Provider::Rule(no_notebooks)),
    (
        "no-artifacts-in-package",
        Provider::Rule(no_artifacts_in_package),
    ),
    (IO_ERROR, Provider::Engine),
    (UNUSED, Provider::Engine),
    (INVALID, Provider::Engine),
];

/// The rules that read `Context::code_dirs`, so a command can say which directories they see.
const USES_CODE_DIRS: [&str; 3] = [
    "no-tests-in-package",
    "no-notebooks",
    "no-artifacts-in-package",
];

pub fn uses_code_dirs(slug: &str) -> bool {
    USES_CODE_DIRS.contains(&slug)
}

fn no_tests_in_package(_options: &Options, context: &Context) -> Box<dyn NativeRule> {
    Box::new(NoTestsInPackage {
        code_dirs: context.code_dirs.clone(),
    })
}

fn no_notebooks(_options: &Options, context: &Context) -> Box<dyn NativeRule> {
    Box::new(NoNotebooks {
        code_dirs: context.code_dirs.clone(),
    })
}

fn no_artifacts_in_package(_options: &Options, context: &Context) -> Box<dyn NativeRule> {
    Box::new(NoArtifactsInPackage {
        code_dirs: context.code_dirs.clone(),
    })
}

fn file_size_guard(options: &Options, _context: &Context) -> Box<dyn NativeRule> {
    let defaults = FileSizeGuard::default();
    Box::new(FileSizeGuard {
        warn_at: threshold(options, "warn", defaults.warn_at),
        error_at: threshold(options, "max", defaults.error_at),
    })
}

fn no_utils(_options: &Options, _context: &Context) -> Box<dyn NativeRule> {
    Box::new(NoUtils)
}

fn threshold(options: &Options, name: &str, default: usize) -> usize {
    options
        .get(name)
        .and_then(toml::Value::as_integer)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(default)
}

/// The native rule `slug` names, set up with `options` and what the project says. `None` when
/// the engine provides the rule itself or when nothing implements it.
pub fn build(slug: &str, options: &Options, context: &Context) -> Option<Box<dyn NativeRule>> {
    NATIVES.iter().find_map(|(name, provider)| match provider {
        Provider::Rule(make) if *name == slug => Some(make(options, context)),
        _ => None,
    })
}

#[derive(Debug, PartialEq, Eq)]
pub struct MissingImplementations {
    pub slugs: Vec<String>,
}

impl fmt::Display for MissingImplementations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the catalog has native rules that this build does not implement: {}",
            self.slugs.join(", ")
        )
    }
}

impl std::error::Error for MissingImplementations {}

pub fn check_catalog(catalog: &Catalog) -> Result<(), MissingImplementations> {
    let slugs: Vec<String> = catalog
        .rules()
        .iter()
        .filter(|rule| {
            rule.implementations
                .iter()
                .any(|implementation| implementation.kind == Kind::Native)
        })
        .filter(|rule| NATIVES.iter().all(|(name, _)| *name != rule.id))
        .map(|rule| rule.id.clone())
        .collect();
    if slugs.is_empty() {
        Ok(())
    } else {
        Err(MissingImplementations { slugs })
    }
}
