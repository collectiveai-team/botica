#![allow(dead_code)]

use botica_catalog::Catalog;
use botica_config::{Config, Reason, Resolution};

pub fn embedded() -> Catalog {
    Catalog::embedded().expect("the built-in catalog loads")
}

pub fn config(text: &str) -> Config {
    Config::from_botica_toml(text).unwrap_or_else(|error| panic!("config:\n{error}"))
}

pub fn resolve(catalog: &Catalog, text: &str) -> Resolution {
    Resolution::new(&config(text), catalog).unwrap_or_else(|error| panic!("resolve:\n{error}"))
}

pub fn active(resolution: &Resolution) -> Vec<String> {
    let mut slugs: Vec<String> = resolution
        .entries()
        .iter()
        .filter(|e| e.active)
        .map(|e| e.rule.id.clone())
        .collect();
    slugs.sort();
    slugs
}

pub fn reason(resolution: &Resolution, slug: &str) -> Reason {
    resolution
        .entry(slug)
        .unwrap_or_else(|| panic!("no rule {slug}"))
        .reason
        .clone()
}
