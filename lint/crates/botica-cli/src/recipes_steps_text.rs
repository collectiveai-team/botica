use std::fmt::Write as _;

use botica_catalog::{Recipe, ResolvedRecipe};

/// The line of `botica recipe list` with the number of steps, when the recipe has any.
pub fn count_line(recipe: &Recipe) -> Option<String> {
    let skills: usize = recipe.skills.iter().map(|step| step.names.len()).sum();
    let (files, sections) = (recipe.files.len(), recipe.sections.len());
    (skills + files + sections > 0)
        .then(|| format!("  skills: {skills}, files: {files}, sections: {sections}\n"))
}

/// The sections of `botica recipe show` for the steps, each one when the recipe has any.
pub fn steps_text(resolved: &ResolvedRecipe) -> String {
    let mut text = String::new();
    if !resolved.skills.is_empty() {
        text.push_str("## Skills\n\n");
        for skill in &resolved.skills {
            let _ = writeln!(
                text,
                "- {} ({}, from {})",
                skill.name, skill.source, skill.from
            );
        }
        text.push('\n');
    }
    if !resolved.files.is_empty() {
        text.push_str("## Files\n\n");
        for file in &resolved.files {
            let _ = writeln!(
                text,
                "- {} (template {}, from {})",
                file.path, file.template, file.from
            );
        }
        text.push('\n');
    }
    if !resolved.sections.is_empty() {
        text.push_str("## Sections\n\n");
        for section in &resolved.sections {
            let _ = writeln!(
                text,
                "- {}, `{}` (template {}, from {})",
                section.path, section.marker, section.template, section.from
            );
        }
        text.push('\n');
    }
    text
}
