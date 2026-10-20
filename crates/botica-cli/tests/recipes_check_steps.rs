//! `botica recipe check` goes through the steps of a recipe: a file exists, the marker of a
//! section is in its file, and a skill is in `skills-lock.json` and in `.agents/skills/`.

mod common;

use common::recipes::{add_recipe, install_skill, lock_skills, project, recipe, SELECT_BASE};
use common::refs::{failed, ok};
use common::{write, write_bytes};

const FILE: &str = "[[files]]\npath = \"prek.toml\"\ntemplate = \"files/prek.toml\"\n";
const SECTION: &str = "[[sections]]\npath = \"AGENTS.md\"\nmarker = \"## Standards\"\n\
                       template = \"sections/standards.md\"\n";
const SKILLS: &str = "[[skills]]\nsource = \"acme/skills\"\nnames = [\"tdd\", \"grilling\"]\n";

#[test]
fn a_file_that_exists_is_ok() {
    let dir = project("steps-file-ok", None);
    add_recipe(&dir, "mine", FILE);
    write(&dir, "prek.toml", "");

    let text = ok(&recipe(&dir, &["check", "mine"]));

    assert_eq!(text, "file prek.toml: ok\nmine: 1 checked, 0 differ\n");
}

#[test]
fn a_file_that_does_not_exist_differs() {
    let dir = project("steps-file-missing", None);
    add_recipe(&dir, "mine", FILE);

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert_eq!(
        text,
        "file prek.toml: differs: does not exist\nmine: 1 checked, 1 differs\n"
    );
}

#[test]
fn a_file_path_that_is_a_directory_differs() {
    let dir = project("steps-file-dir", None);
    add_recipe(&dir, "mine", FILE);
    write(&dir, "prek.toml/inside", "");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("file prek.toml: differs: is not a file\n"),
        "{text}"
    );
}

#[test]
fn a_section_whose_marker_is_in_its_file_is_ok() {
    let dir = project("steps-section-ok", None);
    add_recipe(&dir, "mine", SECTION);
    write(&dir, "AGENTS.md", "# Notes\n\n## Standards\n\n- one\n");

    let text = ok(&recipe(&dir, &["check", "mine"]));

    assert_eq!(
        text,
        "section AGENTS.md `## Standards`: ok\nmine: 1 checked, 0 differ\n"
    );
}

#[test]
fn a_section_whose_file_does_not_exist_differs() {
    let dir = project("steps-section-no-file", None);
    add_recipe(&dir, "mine", SECTION);

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("section AGENTS.md `## Standards`: differs: does not exist\n"),
        "{text}"
    );
}

#[test]
fn a_section_whose_marker_is_not_in_its_file_differs() {
    let dir = project("steps-section-no-marker", None);
    add_recipe(&dir, "mine", SECTION);
    write(&dir, "AGENTS.md", "# Notes\n\n## standards\n");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("section AGENTS.md `## Standards`: differs: marker not in the file\n"),
        "{text}"
    );
}

#[test]
fn a_marker_is_found_anywhere_in_the_file() {
    let dir = project("steps-section-inside", None);
    add_recipe(&dir, "mine", SECTION);
    write(&dir, "AGENTS.md", "### Standards for this repo\n");

    let text = ok(&recipe(&dir, &["check", "mine"]));

    assert!(
        text.contains("section AGENTS.md `## Standards`: ok\n"),
        "{text}"
    );
}

#[test]
fn a_section_file_that_cannot_be_read_differs() {
    let dir = project("steps-section-unreadable", None);
    add_recipe(&dir, "mine", SECTION);
    write_bytes(&dir, "AGENTS.md", &[0xff, 0xfe, 0x00]);

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("section AGENTS.md `## Standards`: differs: cannot read the file"),
        "{text}"
    );
}

#[test]
fn a_section_path_that_is_a_directory_differs() {
    let dir = project("steps-section-dir", None);
    add_recipe(&dir, "mine", SECTION);
    write(&dir, "AGENTS.md/inside", "");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("section AGENTS.md `## Standards`: differs: is not a file\n"),
        "{text}"
    );
}

#[test]
fn a_skill_in_the_lock_and_in_the_skills_folder_is_ok() {
    let dir = project("steps-skill-ok", None);
    add_recipe(&dir, "mine", SKILLS);
    lock_skills(&dir, &["tdd", "grilling"]);
    install_skill(&dir, "tdd");
    install_skill(&dir, "grilling");

    let text = ok(&recipe(&dir, &["check", "mine"]));

    assert_eq!(
        text,
        "skill tdd: ok\nskill grilling: ok\nmine: 2 checked, 0 differ\n"
    );
}

#[test]
fn a_skill_that_the_lock_does_not_list_differs() {
    let dir = project("steps-skill-not-locked", None);
    add_recipe(&dir, "mine", SKILLS);
    lock_skills(&dir, &["grilling"]);
    install_skill(&dir, "tdd");
    install_skill(&dir, "grilling");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("skill tdd: differs: not in `skills-lock.json`\n"),
        "{text}"
    );
    assert!(text.contains("skill grilling: ok\n"), "{text}");
    assert!(text.ends_with("mine: 2 checked, 1 differs\n"), "{text}");
}

#[test]
fn a_skill_that_is_not_in_the_skills_folder_differs() {
    let dir = project("steps-skill-not-installed", None);
    add_recipe(&dir, "mine", SKILLS);
    lock_skills(&dir, &["tdd", "grilling"]);
    install_skill(&dir, "grilling");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("skill tdd: differs: not in `.agents/skills/`\n"),
        "{text}"
    );
    assert!(text.contains("skill grilling: ok\n"), "{text}");
}

#[test]
fn a_skill_that_is_in_neither_says_both() {
    let dir = project("steps-skill-neither", None);
    add_recipe(&dir, "mine", SKILLS);
    lock_skills(&dir, &["other"]);

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains(
            "skill tdd: differs: not in `skills-lock.json` and not in `.agents/skills/`\n"
        ),
        "{text}"
    );
}

#[test]
fn a_project_without_a_lock_says_so_for_each_skill() {
    let dir = project("steps-skill-no-lock", None);
    add_recipe(&dir, "mine", SKILLS);
    install_skill(&dir, "tdd");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("skill tdd: differs: no `skills-lock.json`\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "skill grilling: differs: no `skills-lock.json` and not in `.agents/skills/`\n"
        ),
        "{text}"
    );
}

#[test]
fn a_lock_that_is_not_a_manifest_of_skills_differs() {
    let dir = project("steps-skill-bad-lock", None);
    add_recipe(&dir, "mine", SKILLS);
    install_skill(&dir, "tdd");
    install_skill(&dir, "grilling");
    let broken = ["not json", "[]", "{\"version\": 1}", "{\"skills\": []}"];

    for text in broken {
        write(&dir, "skills-lock.json", text);

        let out = failed(&recipe(&dir, &["check", "mine"]), 1);

        assert!(
            out.contains("skill tdd: differs: `skills-lock.json` is not a manifest of skills\n"),
            "{text}: {out}"
        );
    }
}

#[test]
fn a_skills_folder_without_the_skill_file_does_not_count_as_installed() {
    let dir = project("steps-skill-empty-dir", None);
    add_recipe(&dir, "mine", SKILLS);
    lock_skills(&dir, &["tdd", "grilling"]);
    write(&dir, ".agents/skills/tdd/notes.md", "");
    install_skill(&dir, "grilling");

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    assert!(
        text.contains("skill tdd: differs: not in `.agents/skills/`\n"),
        "{text}"
    );
}

#[test]
fn the_steps_come_after_the_rules_and_the_exports_files_then_sections_then_skills() {
    let dir = project("steps-order", None);
    let toml = format!(
        "rules = [\"ghost\"]\njudgment = [\"phantom\"]\nexports = [\"agents-md\"]\n{SKILLS}{SECTION}{FILE}"
    );
    add_recipe(&dir, "mine", &toml);

    let text = failed(&recipe(&dir, &["check", "mine"]), 1);

    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle}: {text}"))
    };
    let order = [
        at("rule ghost"),
        at("judgment rule phantom"),
        at("export agents-md"),
        at("file prek.toml"),
        at("section AGENTS.md"),
        at("skill tdd"),
        at("skill grilling"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{text}");
    assert!(text.ends_with("mine: 7 checked, 7 differ\n"), "{text}");
}

#[test]
fn the_steps_of_what_a_recipe_extends_are_checked() {
    let dir = project("steps-extends", None);
    add_recipe(&dir, "parent", &format!("{SKILLS}{FILE}"));
    add_recipe(&dir, "child", "extends = [\"parent\"]\n");

    let text = failed(&recipe(&dir, &["check", "child"]), 1);

    assert_eq!(
        text,
        "file prek.toml: differs: does not exist\n\
         skill tdd: differs: no `skills-lock.json` and not in `.agents/skills/`\n\
         skill grilling: differs: no `skills-lock.json` and not in `.agents/skills/`\n\
         child: 3 checked, 3 differ\n"
    );
}

#[test]
fn the_paths_of_the_steps_are_from_the_root_of_the_project() {
    let dir = project("steps-root", None);
    add_recipe(&dir, "mine", &format!("{SKILLS}{SECTION}{FILE}"));
    write(&dir, "prek.toml", "");
    write(&dir, "AGENTS.md", "## Standards\n");
    lock_skills(&dir, &["tdd", "grilling"]);
    install_skill(&dir, "tdd");
    install_skill(&dir, "grilling");
    write(&dir, "deep/er/keep", "");

    let text = ok(&recipe(&dir.join("deep/er"), &["check", "mine"]));

    assert!(text.ends_with("mine: 4 checked, 0 differ\n"), "{text}");
}

#[test]
fn the_built_in_base_checks_its_skill_its_file_and_its_section() {
    let dir = project("steps-base", None);

    let text = failed(
        &recipe(&dir, &["check", "base", "--select", SELECT_BASE]),
        1,
    );

    assert!(text.starts_with("rule file-size-guard: ok\n"), "{text}");
    assert!(
        text.contains("file prek.toml: differs: does not exist\n"),
        "{text}"
    );
    assert!(
        text.contains("section AGENTS.md `## Botica rules`: differs: does not exist\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "skill ask-user: differs: no `skills-lock.json` and not in `.agents/skills/`\n"
        ),
        "{text}"
    );
}
