//! `botica recipe install` with the recipes of the agents that are built into the binary.
#![cfg(unix)]

mod common;

use std::fs;

use common::recipes::{project, recipe};
use common::refs::ok;

#[test]
fn the_recipe_of_opencode_writes_its_config() {
    let dir = project("agent-opencode", None);

    let text = ok(&recipe(&dir, &["install", "agent-opencode"]));

    assert_eq!(
        text,
        "file opencode.jsonc: added\nagent-opencode: 1 added, 0 ok, 0 pending\n"
    );
    assert!(fs::read_to_string(dir.join("opencode.jsonc"))
        .expect("read")
        .contains("$schema"));
}

#[test]
fn the_recipe_of_claude_code_writes_its_settings_and_links_the_instructions_and_the_skills() {
    let dir = project("agent-claude", None);

    let text = ok(&recipe(&dir, &["install", "agent-claude-code"]));

    assert_eq!(
        text,
        "file .claude/settings.json: added\nlink CLAUDE.md: added\nlink .claude/skills: added\n\
         agent-claude-code: 3 added, 0 ok, 0 pending\n"
    );
    assert_eq!(
        fs::read_link(dir.join("CLAUDE.md"))
            .expect("link")
            .display()
            .to_string(),
        "AGENTS.md"
    );
    assert_eq!(
        fs::read_link(dir.join(".claude/skills"))
            .expect("link")
            .display()
            .to_string(),
        "../.agents/skills"
    );
    let checked = ok(&recipe(&dir, &["check", "agent-claude-code"]));
    assert!(
        checked.ends_with("agent-claude-code: 3 checked, 0 differ\n"),
        "{checked}"
    );
}

#[test]
fn the_recipe_of_codex_has_nothing_to_write() {
    let dir = project("agent-codex", None);

    let text = ok(&recipe(&dir, &["install", "agent-codex"]));

    assert_eq!(text, "agent-codex: 0 added, 0 ok, 0 pending\n");
}

#[test]
fn base_creates_agents_md_with_the_workspace_defaults_and_the_standards_index() {
    let dir = project("agent-base-md", None);
    let output = common::recipes::with_npx(&dir, "exit 0", &["install", "base"]);
    let text = String::from_utf8_lossy(&output.stdout).into_owned();

    assert!(
        text.contains("section AGENTS.md `## Repo Workspace Defaults`: added: created the file\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "section AGENTS.md `## Engineering Standards`: added: appended to the file\n"
        ),
        "{text}"
    );
    let agents = fs::read_to_string(dir.join("AGENTS.md")).expect("read");
    assert!(
        agents.starts_with("## Repo Workspace Defaults\n"),
        "{agents}"
    );
    assert!(agents.contains("\n## Engineering Standards\n"), "{agents}");
}

#[test]
fn the_skills_are_installed_for_opencode_when_claude_code_reads_them_through_the_link() {
    let dir = project("agent-claude-skills", None);

    let output = common::recipes::with_npx(
        &dir,
        common::recipes::NPX_INSTALLS,
        &["install", "agent-claude-code", "base"],
    );

    assert!(output.status.success());
    let calls = common::recipes::npx_calls(&dir);
    assert!(!calls.is_empty());
    assert!(
        calls
            .iter()
            .all(|call| call.contains(" --agent opencode --yes ")),
        "{calls:?}"
    );
    assert!(fs::read_link(dir.join(".claude/skills")).is_ok());
    assert!(dir.join(".agents/skills/tdd/SKILL.md").is_file());
}

#[test]
fn the_recipes_of_opencode_and_codex_choose_their_agent() {
    for (name, agent) in [("agent-opencode", "opencode"), ("agent-codex", "codex")] {
        let dir = project(&format!("agent-skills-{agent}"), None);
        common::write(
            &dir,
            "recipes/withskill.toml",
            &format!("extends = [\"{name}\"]\n[[skills]]\nsource = \"a/b\"\nnames = [\"x\"]\n"),
        );

        let output = common::recipes::with_npx(
            &dir,
            common::recipes::NPX_INSTALLS,
            &["install", "withskill"],
        );

        assert!(output.status.success());
        assert_eq!(
            common::recipes::npx_calls(&dir),
            [format!(
                "skills add a/b --agent {agent} --yes --json --skill x"
            )]
        );
    }
}
