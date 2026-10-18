//! `botica recipe install` appends a section to a TOML file only when the file stays valid TOML.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::recipes::{add_recipe, project, recipe, template};
use common::refs::{failed, ok};
use common::write;

const SECTION: &str = "[[sections]]\npath = \"pyproject.toml\"\nmarker = \"# MARK: Ruff\"\n\
                       template = \"sections/ruff.toml\"\n";
const TEXT: &str = "# MARK: Ruff\n[tool.ruff]\nline-length = 100\n";

fn setup(name: &str) -> PathBuf {
    let dir = project(name, None);
    add_recipe(&dir, "mine", SECTION);
    template(&dir, "sections/ruff.toml", TEXT);
    dir
}

fn read(dir: &Path) -> String {
    fs::read_to_string(dir.join("pyproject.toml")).expect("read the file")
}

#[test]
fn a_section_that_keeps_the_file_valid_is_appended() {
    let dir = setup("toml-valid");
    write(&dir, "pyproject.toml", "[project]\nname = \"demo\"\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(
        text.contains("section pyproject.toml `# MARK: Ruff`: added: appended to the file\n"),
        "{text}"
    );
    assert_eq!(
        read(&dir),
        "[project]\nname = \"demo\"\n\n# MARK: Ruff\n[tool.ruff]\nline-length = 100\n"
    );
}

#[test]
fn a_section_that_would_define_a_table_twice_is_pending_and_the_file_is_not_touched() {
    let dir = setup("toml-twice");
    write(&dir, "pyproject.toml", "[tool.ruff]\nline-length = 88\n");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains(
            "section pyproject.toml `# MARK: Ruff`: pending: appending the section would make \
             the file invalid TOML\n"
        ),
        "{text}"
    );
    assert!(text.ends_with("mine: 0 added, 0 ok, 1 pending\n"), "{text}");
    assert_eq!(read(&dir), "[tool.ruff]\nline-length = 88\n");
}

#[test]
fn a_toml_file_that_is_not_valid_toml_is_pending_and_the_file_is_not_touched() {
    let dir = setup("toml-broken");
    write(&dir, "pyproject.toml", "this is = not [ toml");

    let output = recipe(&dir, &["install", "mine"]);

    // botica reads `pyproject.toml` to load the project, so the command stops there.
    let text = failed(&output, 2);
    assert!(text.contains("pyproject.toml"), "{text}");
    assert_eq!(read(&dir), "this is = not [ toml");
}

#[test]
fn a_file_that_is_not_toml_is_appended_to_without_looking_at_its_text() {
    let dir = project("toml-md", None);
    add_recipe(
        &dir,
        "mine",
        "[[sections]]\npath = \"NOTES.md\"\nmarker = \"## S\"\ntemplate = \"sections/s.md\"\n",
    );
    template(&dir, "sections/s.md", "## S\n[[[ not toml\n");
    write(&dir, "NOTES.md", "[[[ also not toml\n");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.contains("added: appended to the file\n"), "{text}");
}

#[test]
fn a_toml_file_other_than_pyproject_is_checked_too() {
    let dir = project("toml-prek", None);
    add_recipe(
        &dir,
        "mine",
        "[[sections]]\npath = \"prek.toml\"\nmarker = \"# MARK: x\"\ntemplate = \"sections/x.toml\"\n",
    );
    template(&dir, "sections/x.toml", "# MARK: x\n[[repos\nrepo = 1\n");
    write(&dir, "prek.toml", "# MARK: y\n");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("pending: appending the section would make the file invalid TOML\n"),
        "{text}"
    );
    assert_eq!(
        fs::read_to_string(dir.join("prek.toml")).expect("read"),
        "# MARK: y\n"
    );
}

#[test]
fn a_new_toml_file_is_written_with_the_section() {
    let dir = setup("toml-new");

    ok(&recipe(&dir, &["install", "mine"]));

    assert_eq!(read(&dir), TEXT);
}

#[test]
fn a_toml_file_that_is_already_invalid_is_pending_and_the_file_is_not_touched() {
    let dir = project("toml-prek-broken", None);
    add_recipe(
        &dir,
        "mine",
        "[[sections]]\npath = \"prek.toml\"\nmarker = \"# MARK: x\"\ntemplate = \"sections/x.toml\"\n",
    );
    template(&dir, "sections/x.toml", "# MARK: x\n[[repos]]\n");
    write(&dir, "prek.toml", "= broken");

    let text = failed(&recipe(&dir, &["install", "mine"]), 1);

    assert!(
        text.contains("section prek.toml `# MARK: x`: pending: is not valid TOML\n"),
        "{text}"
    );
    assert_eq!(
        fs::read_to_string(dir.join("prek.toml")).expect("read"),
        "= broken"
    );
}

#[test]
fn the_check_looks_at_the_text_with_the_blank_line_that_separates_the_section() {
    let dir = project("toml-no-newline", None);
    add_recipe(
        &dir,
        "mine",
        "[[sections]]\npath = \"prek.toml\"\nmarker = \"b = 2\"\ntemplate = \"sections/b.toml\"\n",
    );
    template(&dir, "sections/b.toml", "b = 2\n");
    write(&dir, "prek.toml", "a = 1");

    let text = ok(&recipe(&dir, &["install", "mine"]));

    assert!(text.contains("added: appended to the file\n"), "{text}");
    assert_eq!(
        fs::read_to_string(dir.join("prek.toml")).expect("read"),
        "a = 1\n\nb = 2\n"
    );
}
