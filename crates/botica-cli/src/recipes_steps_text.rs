use std::fmt::Write as _;

use botica_catalog::{Recipe, ResolvedRecipe};

/// The line of `botica recipe list` with the number of steps of each kind that the recipe has.
pub fn count_line(recipe: &Recipe) -> Option<String> {
    let skills: usize = recipe.skills.iter().map(|step| step.names.len()).sum();
    let gitignore: usize = recipe
        .gitignore
        .iter()
        .map(|step| step.lines.len() + step.unignore.len())
        .sum();
    let counts = [
        ("agents", recipe.agents.len()),
        ("skills", skills),
        ("files", recipe.files.len()),
        ("sections", recipe.sections.len()),
        ("links", recipe.links.len()),
        ("gitignore", gitignore),
        (
            "tools",
            recipe.tools.iter().map(|step| step.packages.len()).sum(),
        ),
    ];
    let shown: Vec<String> = counts
        .iter()
        .filter(|(_, count)| *count > 0)
        .map(|(kind, count)| format!("{kind}: {count}"))
        .collect();
    (!shown.is_empty()).then(|| format!("  {}\n", shown.join(", ")))
}

/// The sections of `botica recipe show` for the steps, each one when the recipe has any.
pub fn steps_text(resolved: &ResolvedRecipe) -> String {
    let mut text = String::new();
    if !resolved.agents.is_empty() {
        text.push_str("## Agents\n\n");
        for agent in &resolved.agents {
            let _ = writeln!(text, "- {} (from {})", agent.id, agent.from);
        }
        text.push('\n');
    }
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
    if !resolved.links.is_empty() {
        text.push_str("## Links\n\n");
        for link in &resolved.links {
            let _ = writeln!(
                text,
                "- {} -> {} (from {})",
                link.path, link.target, link.from
            );
        }
        text.push('\n');
    }
    for (title, items) in [
        ("Gitignore", &resolved.gitignore),
        ("Unignore", &resolved.unignore),
    ] {
        if !items.is_empty() {
            let _ = writeln!(text, "## {title}\n");
            for item in items {
                let _ = writeln!(text, "- {} (from {})", item.id, item.from);
            }
            text.push('\n');
        }
    }
    if !resolved.tools.is_empty() {
        text.push_str("## Tools\n\n");
        for tool in &resolved.tools {
            let _ = writeln!(
                text,
                "- {} ({}, group {}, from {})",
                tool.package, tool.manager, tool.group, tool.from
            );
        }
        text.push('\n');
    }
    text
}
