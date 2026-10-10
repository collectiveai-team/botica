use std::collections::HashMap;

use botica_config::{Entry, Resolution};
use botica_core::{FileSizeGuard, NativeRule, NoUtils};

pub struct Natives {
    pub rules: Vec<Box<dyn NativeRule>>,
    pub index: HashMap<String, usize>,
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
