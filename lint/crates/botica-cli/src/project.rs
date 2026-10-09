use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use botica_catalog::Catalog;
use botica_config::{discover, load_file, Entry, Loaded, Resolution};
use botica_core::{language_of, FileSizeGuard, NativePlanner, NativeRule, NoUtils};

pub struct Project {
    pub loaded: Loaded,
    pub resolution: Resolution,
}

pub struct Natives {
    pub rules: Vec<Box<dyn NativeRule>>,
    pub index: HashMap<String, usize>,
}

pub fn load(config: Option<&Path>) -> Result<Project> {
    let loaded = match config {
        Some(path) => load_file(path)?,
        None => discover(&std::env::current_dir().context("cannot read the working directory")?)?,
    };
    let catalog = Catalog::embedded()?;
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

pub fn planner(
    resolution: Resolution,
    index: HashMap<String, usize>,
    prefix: Option<String>,
) -> NativePlanner {
    Box::new(move |path| {
        let from_root: Cow<'_, str> = match &prefix {
            Some(prefix) => Cow::Owned(format!("{prefix}/{path}")),
            None => Cow::Borrowed(path),
        };
        resolution
            .rules_for(&from_root, language_of(path))
            .iter()
            .filter_map(|file_rule| index.get(file_rule.rule.id.as_str()).copied())
            .collect()
    })
}
