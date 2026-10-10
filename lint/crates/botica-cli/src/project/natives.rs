use std::collections::HashMap;

use botica_config::Resolution;
use botica_core::{registry, NativeRule};

pub struct Natives {
    pub rules: Vec<Box<dyn NativeRule>>,
    pub index: HashMap<String, usize>,
}

pub fn natives(resolution: &Resolution) -> Natives {
    let mut rules = Vec::new();
    let mut index = HashMap::new();
    for entry in resolution.entries().iter().filter(|entry| entry.active) {
        if let Some(rule) = registry::build(&entry.rule.id, &entry.options) {
            index.insert(entry.rule.id.clone(), rules.len());
            rules.push(rule);
        }
    }
    Natives { rules, index }
}
