use std::collections::BTreeMap;
use std::path::Path;

use botica_catalog::{Implementation, Kind, Rule, Runtime, Severity, Status};
use botica_config::{Entry, IgnoreOrigin};
use serde::Serialize;

use crate::extension_pool::Launcher;

#[derive(Serialize)]
pub struct IgnoreView<'a> {
    pub glob: &'a str,
    pub origin: IgnoreOrigin,
}

/// One rule as the configuration resolved it, for `botica rules --json`. The CES of the rule is
/// documentation, not part of this public format.
#[derive(Serialize)]
pub struct RuleView<'a> {
    pub rule: &'a str,
    pub code: &'a str,
    pub family: Option<&'a str>,
    pub summary: &'a str,
    pub status: Status,
    pub active: bool,
    /// Why it is or is not active, for people: the wording may change, `active` will not.
    pub reason: String,
    pub severity: Severity,
    pub languages: Vec<&'a str>,
    pub aliases: &'a [String],
    pub tags: &'a [String],
    pub ignores: Vec<IgnoreView<'a>>,
    pub options: &'a BTreeMap<String, toml::Value>,
    pub implementations: Vec<ImplementationView<'a>>,
    /// The active rules that cover a judgment rule (`covered-by`): the judges do not read it.
    pub covered_by: Vec<&'a str>,
}

/// One implementation of a rule: its kind, the program that `botica lint` starts for an
/// extension, and the tool that enforces an external one.
#[derive(Serialize)]
pub struct ImplementationView<'a> {
    pub kind: Kind,
    pub languages: &'a [String],
    pub status: Status,
    pub runtime: Option<Runtime>,
    pub runs: Option<Vec<String>>,
    pub tool: Option<&'a str>,
}

/// How the programs of the extension rules of a project start, and its root, which the paths of
/// the programs are shown from.
pub struct Programs<'a> {
    pub launcher: Launcher,
    pub root: &'a Path,
}

impl Programs<'_> {
    /// The program and arguments that `botica lint` starts for an extension implementation, with
    /// the paths inside the project shown from its root.
    pub fn runs(&self, rule: &Rule, implementation: &Implementation) -> Option<Vec<String>> {
        if implementation.kind != Kind::Extension {
            return None;
        }
        let argv = self.launcher.command(rule, implementation).ok()?;
        Some(
            argv.into_iter()
                .map(|arg| match Path::new(&arg).strip_prefix(self.root) {
                    Ok(inside) if Path::new(&arg).is_absolute() => {
                        inside.to_string_lossy().into_owned()
                    }
                    _ => arg,
                })
                .collect(),
        )
    }
}

impl<'a> ImplementationView<'a> {
    pub fn new(rule: &Rule, implementation: &'a Implementation, programs: &Programs<'_>) -> Self {
        Self {
            kind: implementation.kind,
            languages: &implementation.languages,
            status: rule.status_of(implementation),
            runtime: implementation.runtime,
            runs: programs.runs(rule, implementation),
            tool: implementation.tool.as_deref(),
        }
    }
}

impl<'a> RuleView<'a> {
    pub fn new(entry: &'a Entry, programs: &Programs<'_>, covered_by: Vec<&'a str>) -> Self {
        let rule = &entry.rule;
        Self {
            rule: &rule.id,
            code: &rule.code,
            family: rule.family.as_deref(),
            summary: &rule.summary,
            status: rule.status,
            active: entry.active,
            reason: entry.reason.to_string(),
            severity: entry.severity,
            languages: rule.languages(),
            aliases: &rule.aliases,
            tags: &rule.tags,
            ignores: entry
                .ignore_patterns
                .iter()
                .map(|pattern| IgnoreView {
                    glob: &pattern.glob,
                    origin: pattern.origin,
                })
                .collect(),
            options: &entry.options,
            implementations: rule
                .implementations
                .iter()
                .map(|implementation| ImplementationView::new(rule, implementation, programs))
                .collect(),
            covered_by,
        }
    }
}
