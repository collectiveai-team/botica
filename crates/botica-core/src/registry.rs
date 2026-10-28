use std::collections::BTreeMap;
use std::fmt;

use botica_catalog::{Catalog, Kind};

use crate::agents_refs::AGENTS_MD_REFS_EXIST;
use crate::botica_version::BOTICA_VERSION_ALIGNED;
use crate::commit::{
    ConventionalCommits, MessageRule, NoAiCoauthorship, CONVENTIONAL_COMMITS, NO_AI_COAUTHORSHIP,
};
use crate::complexity::{CognitiveComplexity, COGNITIVE_COMPLEXITY};
use crate::coverage::{NoCoverageExclusion, NO_COVERAGE_EXCLUSION};
use crate::eager_test::{EagerTest, EAGER_TEST};
use crate::env_schema::SETTINGS_ENV_SCHEMA;
use crate::external_config::EXTERNAL_CONFIG;
use crate::external_weakened::EXTERNAL_CONFIG_WEAKENED;
use crate::heavy_imports::{NoEagerHeavyImports, NO_EAGER_HEAVY_IMPORTS};
use crate::import_layers::IMPORT_LAYERS;
use crate::layout::{CodeDir, NoArtifactsInPackage, NoNotebooks, NoTestsInPackage};
use crate::lockfiles::LOCKFILE_COMMITTED;
use crate::native::{FileSizeGuard, NativeRule, NoUtils};
use crate::recipe_conflict::RECIPE_CONFLICT;
use crate::route_auth::{RouteAuth, ROUTE_AUTH};
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
    /// the run once for the whole project (`external-config`, `external-config-weakened`,
    /// `settings-env-schema`, `import-layers`, `recipe-conflict`, `agents-md-refs-exist`,
    /// `botica-version-aligned`).
    Engine,
    /// A rule of scope `commit`, which reads a commit message.
    Message {
        make: fn(&Options) -> Box<dyn MessageRule>,
    },
}

const NATIVES: [(&str, Provider); 23] = [
    (
        CONVENTIONAL_COMMITS,
        Provider::Message {
            make: |options| {
                Box::new(ConventionalCommits {
                    types: strings(options, "types"),
                    exempt: strings(options, "exempt"),
                })
            },
        },
    ),
    (
        NO_AI_COAUTHORSHIP,
        Provider::Message {
            make: |options| {
                Box::new(NoAiCoauthorship {
                    tools: strings(options, "tools"),
                })
            },
        },
    ),
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
    (
        ROUTE_AUTH,
        Provider::Rule {
            make: route_auth,
            reads_code_dirs: false,
        },
    ),
    (
        NO_COVERAGE_EXCLUSION,
        Provider::Rule {
            make: |_, _| Box::new(NoCoverageExclusion),
            reads_code_dirs: false,
        },
    ),
    (
        COGNITIVE_COMPLEXITY,
        Provider::Rule {
            make: cognitive_complexity,
            reads_code_dirs: false,
        },
    ),
    (
        EAGER_TEST,
        Provider::Rule {
            make: eager_test,
            reads_code_dirs: false,
        },
    ),
    (
        NO_EAGER_HEAVY_IMPORTS,
        Provider::Rule {
            make: |options, _| {
                Box::new(NoEagerHeavyImports {
                    modules: strings(options, "modules"),
                })
            },
            reads_code_dirs: false,
        },
    ),
    (IO_ERROR, Provider::Engine),
    (UNUSED, Provider::Engine),
    (INVALID, Provider::Engine),
    (EXTERNAL_CONFIG, Provider::Engine),
    (EXTERNAL_CONFIG_WEAKENED, Provider::Engine),
    (SETTINGS_ENV_SCHEMA, Provider::Engine),
    (IMPORT_LAYERS, Provider::Engine),
    (LOCKFILE_COMMITTED, Provider::Engine),
    (RECIPE_CONFLICT, Provider::Engine),
    (AGENTS_MD_REFS_EXIST, Provider::Engine),
    (BOTICA_VERSION_ALIGNED, Provider::Engine),
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

/// Whether a comment on the line of a finding of `slug`, a rule of project scope, silences it.
/// The findings of the other rules of project scope are about a file or the project, and only the
/// configuration silences them.
pub fn silenced_on_its_line(slug: &str) -> bool {
    slug == EXTERNAL_CONFIG_WEAKENED
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

fn route_auth(options: &Options, _context: &Context) -> Box<dyn NativeRule> {
    Box::new(RouteAuth {
        dependencies: strings(options, "dependencies"),
        public: strings(options, "public"),
    })
}

fn cognitive_complexity(options: &Options, _context: &Context) -> Box<dyn NativeRule> {
    let default = CognitiveComplexity::default().max as usize;
    Box::new(CognitiveComplexity {
        max: u32::try_from(threshold(options, "max", default)).unwrap_or(u32::MAX),
    })
}

fn eager_test(options: &Options, _context: &Context) -> Box<dyn NativeRule> {
    Box::new(EagerTest {
        max: threshold(options, "max", EagerTest::default().max),
    })
}

fn strings(options: &Options, name: &str) -> Vec<String> {
    options
        .get(name)
        .and_then(toml::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
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

/// The rule of scope `commit` that `slug` names, set up with `options`; `None` for any other rule.
pub fn build_message(slug: &str, options: &Options) -> Option<Box<dyn MessageRule>> {
    NATIVES.iter().find_map(|(name, provider)| match provider {
        Provider::Message { make } if *name == slug => Some(make(options)),
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
