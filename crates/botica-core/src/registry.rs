use std::collections::BTreeMap;
use std::fmt;

use botica_catalog::{Catalog, Kind};

use crate::env_schema::SETTINGS_ENV_SCHEMA;
use crate::external_config::EXTERNAL_CONFIG;
use crate::import_layers::IMPORT_LAYERS;
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
    /// Where botica runs inside the project root (`""` at the root).
    pub working_prefix: String,
}

enum Provider {
    Rule {
        make: fn(&Options, &Context) -> Box<dyn NativeRule>,
        /// Whether the rule reads `Context::code_dirs`, so a command can say which directories
        /// it sees and a search for them is only made when one of these is active.
        reads_code_dirs: bool,
    },
    /// The engine reports these itself while it lints (suppression comments, unreadable files), or
    /// the run once for the whole project (`external-config`, `settings-env-schema`,
    /// `import-layers`).
    Engine,
}

const NATIVES: [(&str, Provider); 11] = [
    (
        "file-size-guard",
        Provider::Rule {
            make: file_size_guard,
            reads_code_dirs: false,
        },
    ),
    (
        "no-utils",
        Provider::Rule {
            make: no_utils,
            reads_code_dirs: false,
        },
    ),
    (
        "no-tests-in-package",
        Provider::Rule {
            make: |_, context| over_code_dirs::<NoTestsInPackage>(context),
            reads_code_dirs: true,
        },
    ),
    (
        "no-notebooks",
        Provider::Rule {
            make: |_, context| over_code_dirs::<NoNotebooks>(context),
            reads_code_dirs: true,
        },
    ),
    (
        "no-artifacts-in-package",
        Provider::Rule {
            make: |_, context| over_code_dirs::<NoArtifactsInPackage>(context),
            reads_code_dirs: true,
        },
    ),
    (IO_ERROR, Provider::Engine),
    (UNUSED, Provider::Engine),
    (INVALID, Provider::Engine),
    (EXTERNAL_CONFIG, Provider::Engine),
    (SETTINGS_ENV_SCHEMA, Provider::Engine),
    (IMPORT_LAYERS, Provider::Engine),
];

pub fn uses_code_dirs(slug: &str) -> bool {
    NATIVES.iter().any(|(name, provider)| {
        *name == slug
            && matches!(
                provider,
                Provider::Rule {
                    reads_code_dirs: true,
                    ..
                }
            )
    })
}

fn over_code_dirs<R>(context: &Context) -> Box<dyn NativeRule>
where
    R: NativeRule + From<Vec<CodeDir>> + 'static,
{
    Box::new(R::from(context.code_dirs.clone()))
}

fn file_size_guard(options: &Options, _context: &Context) -> Box<dyn NativeRule> {
    let defaults = FileSizeGuard::default();
    Box::new(FileSizeGuard {
        warn_at: threshold(options, "warn", defaults.warn_at),
        error_at: threshold(options, "max", defaults.error_at),
    })
}

fn no_utils(_options: &Options, context: &Context) -> Box<dyn NativeRule> {
    Box::new(NoUtils {
        working_prefix: context.working_prefix.clone(),
    })
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
        Provider::Rule { make, .. } if *name == slug => Some(make(options, context)),
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
