mod args;
mod natives;
mod paths;
mod policy;

use std::path::Path;

use anyhow::{bail, Context, Result};
use botica_catalog::Catalog;
use botica_config::{discover, load_file, Loaded, Resolution};

use args::{ConfigOption, ScopeArg};

pub use args::ProjectArgs;
pub use natives::natives;
pub use paths::{exclusion, planner, verdicts};
pub use policy::{rule_docs, suppression_policy};

pub struct Project {
    pub loaded: Loaded,
    pub resolution: Resolution,
    /// Whether the selection takes in the syntax rules embedded in the binary. They are not in
    /// the catalog yet, so no selector can name them: they run only when the selection includes
    /// `default` or `all`.
    pub embedded_rules: bool,
}

pub fn load(args: &ProjectArgs) -> Result<Project> {
    let files: Vec<&Path> = args
        .config
        .iter()
        .filter_map(|option| match option {
            ConfigOption::File(path) => Some(path.as_path()),
            ConfigOption::Setting(_) => None,
        })
        .collect();
    let settings: Vec<toml::Table> = args
        .config
        .iter()
        .filter_map(|option| match option {
            ConfigOption::Setting(setting) => Some(setting.clone()),
            ConfigOption::File(_) => None,
        })
        .collect();
    if files.len() > 1 {
        bail!("more than one configuration file was given with --config: keep only one");
    }
    let loaded = match files.first() {
        Some(path) => load_file(path)?,
        None => discover(&std::env::current_dir().context("cannot read the working directory")?)?,
    };
    let mut loaded = loaded.with_inline(&settings)?;
    let catalog = Catalog::embedded()?;
    loaded.config = loaded
        .config
        .with_command_line(&catalog, &args.command_line())?;
    let resolution = Resolution::new(&loaded.config, &catalog)?.only_scope(args.scope.only());
    // The embedded syntax rules look at one file each.
    let embedded_rules = args.scope != ScopeArg::Project
        && loaded
            .config
            .select
            .iter()
            .chain(&loaded.config.extend_select)
            .any(|selector| selector == "default" || selector == "all");
    Ok(Project {
        loaded,
        resolution,
        embedded_rules,
    })
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
        let active: Vec<&str> = self
            .resolution
            .entries()
            .iter()
            .filter(|entry| entry.active)
            .map(|entry| entry.rule.id.as_str())
            .collect();
        format!("{:?}|{}", self.loaded.config, active.join(","))
    }
}
