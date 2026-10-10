use std::collections::HashMap;

use botica_core::registry::{self, Context};
use botica_core::NativeRule;

use super::{packages, Project};

pub struct Natives {
    pub rules: Vec<Box<dyn NativeRule>>,
    pub index: HashMap<String, usize>,
}

pub fn natives(project: &Project) -> Natives {
    let context = Context {
        packages: packages::seen_from(project.packages(), project.working_prefix().as_deref()),
    };
    let mut rules = Vec::new();
    let mut index = HashMap::new();
    for entry in project
        .resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
    {
        if let Some(rule) = registry::build(&entry.rule.id, &entry.options, &context) {
            index.insert(entry.rule.id.clone(), rules.len());
            rules.push(rule);
        }
    }
    Natives { rules, index }
}
