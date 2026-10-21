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
    push_section(&mut text, "Agents", &resolved.agents, |agent| {
        format!("{} (from {})", agent.id, agent.from)
    });
    push_section(&mut text, "Skills", &resolved.skills, |skill| {
        format!("{} ({}, from {})", skill.name, skill.source, skill.from)
    });
    push_section(&mut text, "Files", &resolved.files, |file| {
        format!(
            "{} (template {}, from {})",
            file.path, file.template, file.from
        )
    });
    push_section(&mut text, "Sections", &resolved.sections, |section| {
        format!(
            "{}, `{}` (template {}, from {})",
            section.path, section.marker, section.template, section.from
        )
    });
    push_section(&mut text, "Links", &resolved.links, |link| {
        format!("{} -> {} (from {})", link.path, link.target, link.from)
    });
    for (title, items) in [
        ("Gitignore", &resolved.gitignore),
        ("Unignore", &resolved.unignore),
    ] {
        push_section(&mut text, title, items, |item| {
            format!("{} (from {})", item.id, item.from)
        });
    }
    push_section(&mut text, "Tools", &resolved.tools, |tool| {
        format!(
            "{} ({}, group {}, from {})",
            tool.package, tool.manager, tool.group, tool.from
        )
    });
    text
}

/// Appends the section `title` with one line for each item, or nothing when there is no item.
fn push_section<T>(text: &mut String, title: &str, items: &[T], line: impl Fn(&T) -> String) {
    if items.is_empty() {
        return;
    }
    let _ = writeln!(text, "## {title}\n");
    for item in items {
        let _ = writeln!(text, "- {}", line(item));
    }
    text.push('\n');
}
