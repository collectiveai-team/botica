//! The extension rules of a project, as `botica lint` and `botica server` reach them: which
//! implementations read a file, the command that runs each one, and which findings are the
//! rule's own.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use botica_catalog::{Implementation, Rule, Runtime};
use botica_config::Resolution;
use botica_core::Diagnostic;

use crate::project::extensions_at;

pub struct ExtensionPool {
    resolution: Arc<Resolution>,
    prefix: Option<String>,
    catalog_dirs: BTreeMap<String, PathBuf>,
    project_root: PathBuf,
}

impl ExtensionPool {
    pub const fn new(
        resolution: Arc<Resolution>,
        prefix: Option<String>,
        catalog_dirs: BTreeMap<String, PathBuf>,
        project_root: PathBuf,
    ) -> Self {
        Self {
            resolution,
            prefix,
            catalog_dirs,
            project_root,
        }
    }

    /// Where the processes of the rules run.
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// The extension implementations that read the file at `path`, each with its rule.
    pub fn at(&self, path: &str) -> Vec<(&Rule, &Implementation)> {
        extensions_at(&self.resolution, self.prefix.as_deref(), path)
    }

    /// What runs the implementation: `uv run --script` on the script of a Python rule, `node` on
    /// the script of a Node rule, the command of a rule that names one.
    pub fn command_of(
        &self,
        rule: &Rule,
        implementation: &Implementation,
    ) -> Result<Vec<String>, String> {
        let named = |what: &str| format!("`{}` has no {what}", rule.id);
        match implementation.runtime {
            Some(Runtime::Command) => implementation
                .command
                .clone()
                .ok_or_else(|| named("`command`")),
            Some(runtime) => {
                let script = self
                    .script_of(rule, implementation)
                    .ok_or_else(|| named("script"))?;
                let script = script.to_string_lossy().into_owned();
                Ok(match runtime {
                    Runtime::Python => ["uv", "run", "--script"]
                        .map(str::to_owned)
                        .into_iter()
                        .chain([script])
                        .collect(),
                    _ => vec!["node".to_owned(), script],
                })
            }
            None => Err(named("runtime")),
        }
    }

    /// The script of the rule, in its folder of `<catalog>/<family>/<slug>/`, where the catalog is.
    fn script_of(&self, rule: &Rule, implementation: &Implementation) -> Option<PathBuf> {
        let catalog = rule.catalog.as_deref()?;
        let slug = rule.id.strip_prefix(catalog)?.strip_prefix('/')?;
        let folder = self
            .catalog_dirs
            .get(catalog)?
            .join(rule.family.as_deref()?)
            .join(slug);
        Some(folder.join(implementation.file.as_deref()?))
    }
}

/// The findings that are about the rule. A finding about another rule is left out, and said once.
pub fn own_findings(rule: &Rule, found: Vec<Diagnostic>) -> Vec<Diagnostic> {
    let (own, foreign): (Vec<Diagnostic>, Vec<Diagnostic>) = found
        .into_iter()
        .partition(|finding| finding.rule == rule.id);
    let mut named: Vec<&str> = foreign
        .iter()
        .map(|finding| finding.rule.as_str())
        .collect();
    named.sort_unstable();
    named.dedup();
    for other in named {
        eprintln!(
            "botica: `{}` answered with a finding of `{other}`: left out",
            rule.id
        );
    }
    own
}
