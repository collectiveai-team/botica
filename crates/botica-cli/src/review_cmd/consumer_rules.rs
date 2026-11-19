use std::collections::BTreeMap;

use anyhow::{ensure, Context, Result};
use botica_catalog::{Catalog, SourceFile};
use botica_config::{Config, Resolution};
use botica_core::review::roles::relative_path;
use serde_json::{json, Value};

use crate::review_cmd::tier;

fn settings(assessment: &tier::Assessment) -> Result<Config> {
    let allowed = [
        "select",
        "extend-select",
        "ignore",
        "preview",
        "languages",
        "rules",
        "overrides",
        "external",
        "replace",
        "extend-exclude",
        "extend-include",
        "code-dirs",
        "rule-dirs",
        "catalogs",
    ];
    let mut table = assessment.config.table.clone().unwrap_or_default();
    table.retain(|key, _| allowed.contains(&key));
    toml::Value::Table(table)
        .try_into()
        .context("invalid trusted rule settings")
}

fn directories(
    files: &[String],
    config: &Config,
    prefix: &str,
) -> Result<BTreeMap<String, String>> {
    let mut directories = BTreeMap::new();
    for directory in &config.rule_dirs {
        relative_path(directory)?;
        let directory = format!("{prefix}{directory}/");
        for name in files
            .iter()
            .filter_map(|path| path.strip_prefix(&directory))
            .filter_map(|relative| relative.split_once('/').map(|(name, _)| name))
        {
            let root = format!("{directory}{name}/");
            if let Some(previous) = directories.insert(name.to_owned(), root.clone()) {
                ensure!(previous == root, "duplicate trusted catalog {name}");
            }
        }
    }
    for (name, source) in &config.catalogs {
        relative_path(&source.path)?;
        ensure!(
            directories
                .insert(name.clone(), format!("{prefix}{}/", source.path))
                .is_none(),
            "duplicate trusted catalog {name}"
        );
    }
    Ok(directories)
}

fn catalog_source(path: &str) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    match parts.as_slice() {
        ["catalog.toml" | "history.toml"] | [_, "family.toml"] => true,
        [_, _, name] => {
            matches!(*name, "rule.toml" | "README.md")
                || [".yml", ".py", ".js", ".mjs"]
                    .iter()
                    .any(|suffix| name.ends_with(suffix))
        }
        [_, _, "config", _, ..] => true,
        _ => false,
    }
}

fn catalogs(assessment: &tier::Assessment, config: &Config) -> Result<(Catalog, Vec<Value>)> {
    let files = assessment.git.base_paths()?;
    let prefix = assessment
        .config
        .source
        .rsplit_once('/')
        .map(|(dir, _)| format!("{dir}/"))
        .unwrap_or_default();
    let mut sources = Vec::new();
    let mut fingerprints = Vec::new();
    for (name, directory) in directories(&files, config, &prefix)? {
        let mut count = 0;
        for path in files
            .iter()
            .filter(|path| path.starts_with(&directory) && !path.contains("/fixtures/"))
        {
            let bytes = assessment
                .git
                .blob(path)?
                .context("missing trusted catalog file")?;
            fingerprints
                .push(json!({"path": path, "digest": blake3::hash(&bytes).to_hex().to_string()}));
            let relative = path.strip_prefix(&directory).context("catalog prefix")?;
            // Match read_catalog_files: other tracked files are bound but never executed.
            if catalog_source(relative) {
                sources.push(SourceFile {
                    path: format!("{name}/{relative}"),
                    text: String::from_utf8(bytes).context("catalog file is not UTF-8")?,
                });
            }
            count += 1;
        }
        ensure!(count > 0, "missing trusted catalog {name}");
    }
    Ok((
        Catalog::embedded()?
            .with_projects(&sources)
            .context("invalid trusted catalog")?,
        fingerprints,
    ))
}

pub(super) fn references(
    assessment: &tier::Assessment,
    declarations: &[String],
) -> Result<Vec<Value>> {
    if declarations.is_empty() {
        return Ok(Vec::new());
    }
    let config = settings(assessment)?;
    let (catalog, project_sources) = catalogs(assessment, &config)?;
    let resolution = Resolution::new(&config, &catalog)?;
    let exported: Value = serde_json::from_str(&crate::catalog_json::render(&catalog))?;
    let mut references = Vec::new();
    for declaration in declarations {
        let rule = catalog
            .resolve(declaration)
            .with_context(|| format!("unknown covered rule {declaration}"))?;
        ensure!(
            &rule.id == declaration,
            "covered.rules requires canonical slugs, not codes or aliases"
        );
        ensure!(
            resolution
                .entry(declaration)
                .is_some_and(|entry| entry.active),
            "covered rule {declaration} is not active in trusted configuration"
        );
        let definition = exported["rules"]
            .as_array()
            .context("catalog export has no rules")?
            .iter()
            .find(|definition| definition["rule"] == *declaration)
            .context("missing rule definition")?;
        let fingerprint = json!({"definition": definition, "config": assessment.config.digest,
            "botica_version": env!("CARGO_PKG_VERSION"), "project_sources": project_sources});
        references.push(
            json!({"kind": "rule", "id": declaration, "file": assessment.config.source,
            "digest": blake3::hash(&serde_json::to_vec(&fingerprint)?).to_hex().to_string()}),
        );
    }
    Ok(references)
}
