//! `agents-md-refs-exist` in `botica lint`: the run reads each `AGENTS.md` and `CLAUDE.md` of the
//! project and looks for the paths of their references on disk.

mod common;

use common::{botica, project_in, stdout, write};

const SELECTED: &str = "select = [\"agents-md-refs-exist\"]\npreview = true\n";

#[test]
fn a_reference_to_a_missing_file_is_reported_at_its_line() {
    let dir = project_in("agents-md-refs-exist", "missing", Some(SELECTED));
    write(&dir, "docs/a.md", "# A\n");
    write(
        &dir,
        "AGENTS.md",
        "# Notes\n\nSee @docs/a.md and `@docs/b.md`.\n",
    );

    let output = botica(&dir, &[]);

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("AGENTS.md:3:21: error [agents-md-refs-exist] `@docs/b.md` names no file"),
        "{text}"
    );
    assert!(!text.contains("`@docs/a.md`"), "{text}");
}

#[test]
fn a_missing_path_that_gitignore_leaves_out_is_no_finding() {
    let dir = project_in("agents-md-refs-exist", "ignored", Some(SELECTED));
    write(&dir, ".gitignore", ".agents/skills/\n");
    write(&dir, "AGENTS.md", "Skills: @.agents/skills/tdd/SKILL.md\n");
    assert!(!stdout(&botica(&dir, &[])).contains("agents-md-refs-exist"));

    write(&dir, ".gitignore", "\n");
    assert!(stdout(&botica(&dir, &[])).contains("`@.agents/skills/tdd/SKILL.md` names no file"));
}
