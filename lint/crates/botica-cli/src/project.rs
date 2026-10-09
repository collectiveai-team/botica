use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use botica_catalog::Catalog;
use botica_config::{discover, load_file, CommandLine, Entry, FileRule, Loaded, Resolution};
use botica_core::postprocess::Verdict;
use botica_core::report::RuleDoc;
use botica_core::suppress::Policy;
use botica_core::{
    language_of, Exclusion, FileSizeGuard, NativePlanner, NativeRule, NoUtils, Severity,
};
use clap::Args;

pub struct Project {
    pub loaded: Loaded,
    pub resolution: Resolution,
}

pub struct Natives {
    pub rules: Vec<Box<dyn NativeRule>>,
    pub index: HashMap<String, usize>,
}

/// The options that decide which configuration applies and which rules it selects.
#[derive(Args)]
pub struct ProjectArgs {
    /// Use this configuration file instead of looking for one.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Run only these rules (slug, code, family, prefix or `tag:<t>`; comma-separated or repeated).
    /// Starts from scratch: the `select`, `extend-select` and `ignore` of the file do not apply.
    #[arg(long, value_delimiter = ',')]
    select: Vec<String>,
    /// Switch these rules off, on top of what the file ignores.
    #[arg(long, value_delimiter = ',')]
    ignore: Vec<String>,
    /// Also run the rules whose status is `preview`.
    #[arg(long)]
    preview: bool,
}

impl ProjectArgs {
    fn command_line(&self) -> CommandLine {
        CommandLine {
            select: self.select.clone(),
            ignore: self.ignore.clone(),
            preview: self.preview,
        }
    }
}

pub fn load(args: &ProjectArgs) -> Result<Project> {
    let mut loaded = match args.config.as_deref() {
        Some(path) => load_file(path)?,
        None => discover(&std::env::current_dir().context("cannot read the working directory")?)?,
    };
    let catalog = Catalog::embedded()?;
    loaded.config = loaded
        .config
        .with_command_line(&catalog, &args.command_line())?;
    let resolution = Resolution::new(&loaded.config, &catalog)?;
    Ok(Project { loaded, resolution })
}

impl Project {
    pub fn is_active(&self, slug: &str) -> bool {
        self.resolution
            .entry(slug)
            .is_some_and(|entry| entry.active)
    }

    /// Where the working directory sits inside the project root, when it is not the root itself.
    pub fn working_prefix(&self) -> Option<String> {
        let root = std::fs::canonicalize(&self.loaded.root).ok()?;
        let cwd = std::fs::canonicalize(std::env::current_dir().ok()?).ok()?;
        let inside = cwd.strip_prefix(&root).ok()?;
        (!inside.as_os_str().is_empty()).then(|| {
            inside
                .components()
                .map(|part| part.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
    }

    /// Everything that can change which diagnostics a file gets, for the result cache.
    pub fn fingerprint(&self) -> String {
        format!("{:?}", self.loaded.config)
    }
}

fn threshold(entry: &Entry, name: &str, default: usize) -> usize {
    entry
        .options
        .get(name)
        .and_then(toml::Value::as_integer)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(default)
}

fn native_rule(entry: &Entry) -> Option<Box<dyn NativeRule>> {
    match entry.rule.id.as_str() {
        "file-size-guard" => Some(Box::new(FileSizeGuard {
            warn_at: threshold(entry, "warn", 400),
            error_at: threshold(entry, "max", 700),
        })),
        "no-utils" => Some(Box::new(NoUtils)),
        _ => None,
    }
}

pub fn natives(resolution: &Resolution) -> Natives {
    let mut rules = Vec::new();
    let mut index = HashMap::new();
    for entry in resolution.entries().iter().filter(|entry| entry.active) {
        if let Some(rule) = native_rule(entry) {
            index.insert(entry.rule.id.clone(), rules.len());
            rules.push(rule);
        }
    }
    Natives { rules, index }
}

fn severity(level: botica_catalog::Severity) -> Severity {
    match level {
        botica_catalog::Severity::Error => Severity::Error,
        botica_catalog::Severity::Warning => Severity::Warning,
        botica_catalog::Severity::Info => Severity::Info,
    }
}

/// What the suppression comments may name, and whether the rules about them report.
pub fn suppression_policy(resolution: &Arc<Resolution>) -> Policy {
    let mut policy = Policy::new().with_compat_ast_grep_ignore(resolution.compat_ast_grep_ignore());
    for entry in resolution.entries() {
        let spellings: Vec<&str> = std::iter::once(entry.rule.code.as_str())
            .chain(entry.rule.aliases.iter().map(String::as_str))
            .collect();
        policy = policy.with_rule(&entry.rule.id, &spellings);
    }
    let external = Arc::clone(resolution);
    policy = policy.with_external(move |id| external.is_external(id));
    let reporting = |slug: &str| {
        resolution
            .entry(slug)
            .filter(|entry| entry.active)
            .map(|entry| severity(entry.severity))
    };
    if let Some(level) = reporting("unused-suppression") {
        policy = policy.report_unused(level);
    }
    if let Some(level) = reporting("invalid-suppression") {
        policy = policy.report_invalid(level);
    }
    policy
}

/// The catalog's rules as the output formats describe them.
pub fn rule_docs(resolution: &Resolution) -> Vec<RuleDoc> {
    resolution
        .entries()
        .iter()
        .map(|entry| RuleDoc {
            id: entry.rule.id.clone(),
            code: entry.rule.code.clone(),
            summary: entry.rule.summary.clone(),
            severity: severity(entry.severity),
        })
        .collect()
}

/// A path as the configuration sees it: from the project root, not from where botica runs.
fn from_root<'p>(prefix: Option<&str>, path: &'p str) -> Cow<'p, str> {
    match prefix {
        Some(prefix) => Cow::Owned(format!("{prefix}/{path}")),
        None => Cow::Borrowed(path),
    }
}

pub fn planner(
    resolution: Arc<Resolution>,
    index: HashMap<String, usize>,
    prefix: Option<String>,
) -> NativePlanner {
    Box::new(move |path| {
        resolution
            .rules_for(&from_root(prefix.as_deref(), path), language_of(path))
            .iter()
            .filter_map(|file_rule| index.get(file_rule.rule.id.as_str()).copied())
            .collect()
    })
}

/// What `extend-exclude` leaves out of the walk, or `None` when it names nothing.
pub fn exclusion(resolution: Arc<Resolution>, prefix: Option<String>) -> Option<Exclusion> {
    if !resolution.has_exclusions() {
        return None;
    }
    let (for_entries, for_paths) = (Arc::clone(&resolution), resolution);
    let (entry_prefix, path_prefix) = (prefix.clone(), prefix);
    Some(Exclusion {
        entry: Box::new(move |path, is_dir| {
            for_entries.excludes_entry(&from_root(entry_prefix.as_deref(), path), is_dir)
        }),
        path: Box::new(move |path| {
            for_paths.excludes_path(&from_root(path_prefix.as_deref(), path))
        }),
    })
}

/// The rules that apply at one path, each with the severity the configuration names for it.
type Named<'r> = Vec<(&'r str, Option<Severity>)>;

/// What the configuration says about a rule at a path, for the post-process. A rule of the
/// catalog that does not apply at the path is dropped; one that has a severity named for it
/// there gets it; the rules the catalog does not know (ast-grep's, extensions') are left alone.
pub fn verdicts<'r>(
    resolution: &'r Resolution,
    prefix: Option<&'r str>,
) -> impl FnMut(&str, &str) -> Verdict + 'r {
    let mut current: Option<(String, Named<'r>)> = None;
    move |path, rule| {
        let Some(entry) = resolution.entry(rule) else {
            return Verdict::Keep;
        };
        let at_root = from_root(prefix, path);
        if entry.rule.id == "io-error" {
            return verdict_in(&resolution.rules_for_path(&at_root), &entry.rule.id);
        }
        if current.as_ref().is_none_or(|(known, _)| known != path) {
            let rules = resolution.rules_for(&at_root, language_of(path));
            let found = rules
                .iter()
                .map(|file_rule| {
                    (
                        file_rule.rule.id.as_str(),
                        file_rule.configured.map(severity),
                    )
                })
                .collect();
            current = Some((path.to_owned(), found));
        }
        let (_, found) = current.as_ref().expect("filled just above");
        match found.iter().find(|(id, _)| *id == entry.rule.id) {
            Some((_, Some(level))) => Verdict::Report(*level),
            Some((_, None)) => Verdict::Keep,
            None => Verdict::Drop,
        }
    }
}

fn verdict_in(rules: &[FileRule<'_>], id: &str) -> Verdict {
    match rules.iter().find(|file_rule| file_rule.rule.id == id) {
        Some(FileRule {
            configured: Some(level),
            ..
        }) => Verdict::Report(severity(*level)),
        Some(_) => Verdict::Keep,
        None => Verdict::Drop,
    }
}
