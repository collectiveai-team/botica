//! The extension rules of a project, as `botica lint` and `botica server` reach them: which
//! implementations read a file, the command that runs each one, and which findings are the
//! rule's own. `lint` starts the process of a rule, sends it the files and ends it; the server
//! keeps it for the session of an editor.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use botica_catalog::{Implementation, Rule, Runtime, Scope};
use botica_config::Resolution;
use botica_core::suppress::Policy;
use botica_core::{postprocess, Diagnostic, ExtensionRun};
use botica_lsp::{Extensions, Process};

use crate::project::{self, extensions_at};

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

/// The rules of the server are named by the id of the rule, and `<id>#<n>` for the implementation
/// number `n` when it is not the first: each implementation is a process of its own.
fn key_of(rule: &Rule, implementation: &Implementation) -> String {
    let position = rule
        .implementations
        .iter()
        .position(|candidate| std::ptr::eq(candidate, implementation))
        .unwrap_or_default();
    if position == 0 {
        rule.id.clone()
    } else {
        format!("{}#{position}", rule.id)
    }
}

fn split_key(key: &str) -> (&str, usize) {
    key.rsplit_once('#')
        .and_then(|(id, position)| Some((id, position.parse().ok()?)))
        .unwrap_or((key, 0))
}

impl Extensions for ExtensionPool {
    /// The rules of file scope: a rule of project scope reads the whole project, not a buffer.
    fn at(&self, path: &str) -> Vec<String> {
        ExtensionPool::at(self, path)
            .into_iter()
            .filter(|(rule, _)| rule.scope != Some(Scope::Project))
            .map(|(rule, implementation)| key_of(rule, implementation))
            .collect()
    }

    fn start(&self, key: &str) -> Result<Box<dyn Process + '_>, String> {
        let (id, position) = split_key(key);
        let rule = &self
            .resolution
            .entry(id)
            .ok_or_else(|| format!("`{id}` is not a rule of the project"))?
            .rule;
        let implementation = rule
            .implementations
            .get(position)
            .ok_or_else(|| format!("`{id}` has no implementation {position}"))?;
        let argv = self.command_of(rule, implementation)?;
        let run = ExtensionRun::start_in(&argv, Some(&self.project_root))
            .map_err(|error| format!("{error:#}"))?;
        Ok(Box::new(Live {
            pool: self,
            rule,
            run,
            policy: project::suppression_policy(&self.resolution),
        }))
    }
}

/// The process of one rule, alive for the session of an editor.
struct Live<'a> {
    pool: &'a ExtensionPool,
    rule: &'a Rule,
    run: ExtensionRun,
    policy: Policy,
}

impl Process for Live<'_> {
    /// What the rule finds in the buffer, as `botica lint` would report it: the findings of the
    /// rule alone, what the comments of the buffer leave of them, with the severity and the
    /// scope of the configuration.
    fn lint(&mut self, path: &str, text: &str) -> Result<Vec<Diagnostic>, String> {
        let found = self
            .run
            .lint_all([(path, text)], 1)
            .map_err(|error| format!("{error:#}"))?;
        let mut found = own_findings(self.rule, found);
        self.policy.silence(path, text, &mut found);
        Ok(postprocess::apply(
            found,
            project::verdicts(&self.pool.resolution, self.pool.prefix.as_deref()),
        ))
    }

    fn finish(self: Box<Self>) {
        let _ = self.run.finish();
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
