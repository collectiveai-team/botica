use std::collections::HashMap;

use botica_core::registry::{self, Context};
use botica_core::NativeRule;

use super::Project;

pub struct Natives {
    pub rules: Vec<Box<dyn NativeRule>>,
    pub index: HashMap<String, usize>,
}

pub fn natives(project: &Project) -> Natives {
    let context = Context {
        code_dirs: project.code_dirs_for_the_rules().to_vec(),
        working_prefix: project.working_prefix().unwrap_or_default(),
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
