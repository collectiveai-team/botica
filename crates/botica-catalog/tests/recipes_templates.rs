//! The templates of a recipe: the files under `files/` and `sections/` that its steps name.

mod common;

use botica_catalog::{CatalogError, Recipes, SourceFile};
use common::file;

fn recipe(id: &str, toml: &str) -> SourceFile {
    file(&format!("{id}.toml"), toml)
}

fn project(files: Vec<SourceFile>) -> Result<Recipes, CatalogError> {
    Recipes::from_files(&files)
}

fn rejected(files: Vec<SourceFile>) -> String {
    project(files).expect_err("rejected").to_string()
}

const FILE_STEP: &str = "[[files]]\npath = \"prek.toml\"\ntemplate = \"files/prek.toml\"\n";
const SECTION_STEP: &str =
    "[[sections]]\npath = \"AGENTS.md\"\nmarker = \"## M\"\ntemplate = \"sections/m.md\"\n";

#[test]
fn a_recipe_loads_when_the_template_of_each_step_is_in_the_folder() {
    let loaded = project(vec![
        recipe("base", &format!("{FILE_STEP}\n{SECTION_STEP}")),
        file("files/prek.toml", "hooks\n"),
        file("sections/m.md", "## M\n"),
    ])
    .expect("loads");

    assert_eq!(loaded.template("base", "files/prek.toml"), Some("hooks\n"));
    assert_eq!(loaded.template("base", "sections/m.md"), Some("## M\n"));
    assert_eq!(loaded.template("base", "files/other.toml"), None);
    assert_eq!(loaded.template("nope", "files/prek.toml"), None);
}

#[test]
fn a_template_may_be_at_any_depth_and_may_be_unused() {
    let loaded = project(vec![
        recipe("bare", ""),
        file("files/github/workflows/ci.yml", "on: push\n"),
        file("sections/notes.md", "notes\n"),
    ])
    .expect("loads");

    assert_eq!(loaded.recipes().len(), 1);
}

#[test]
fn a_step_whose_template_does_not_exist_is_rejected_and_names_the_recipe_file() {
    let files = rejected(vec![recipe("x", FILE_STEP)]);
    assert!(files.contains("x.toml"), "{files}");
    assert!(
        files.contains(
            "the `files` step for `prek.toml` names the template `files/prek.toml`, \
             which does not exist"
        ),
        "{files}"
    );

    let sections = rejected(vec![recipe("x", SECTION_STEP)]);
    assert!(
        sections.contains(
            "the `sections` step for `AGENTS.md` names the template `sections/m.md`, \
             which does not exist"
        ),
        "{sections}"
    );
}

#[test]
fn a_template_is_under_the_folder_of_its_kind_of_step() {
    let files = rejected(vec![
        recipe(
            "x",
            "[[files]]\npath = \"a\"\ntemplate = \"sections/m.md\"\n",
        ),
        file("sections/m.md", "m"),
    ]);
    assert!(
        files.contains("the `files` step for `a` names the template `sections/m.md`, which is not under `files/`"),
        "{files}"
    );

    let sections = rejected(vec![
        recipe(
            "x",
            "[[sections]]\npath = \"a\"\nmarker = \"m\"\ntemplate = \"files/p.toml\"\n",
        ),
        file("files/p.toml", "p"),
    ]);
    assert!(
        sections.contains("the `sections` step for `a` names the template `files/p.toml`, which is not under `sections/`"),
        "{sections}"
    );
}

#[test]
fn a_file_that_is_not_a_recipe_or_a_template_is_rejected() {
    for path in [
        "nested/c.toml",
        "docs/x.md",
        "files",
        "files/",
        "files//x",
        "files/./x",
        "files/a/../x",
        "sections/",
        "other/files/x",
        "notes.txt",
    ] {
        let message = rejected(vec![file(path, "")]);

        assert!(message.contains(path), "{path}: {message}");
        assert!(
            message.contains("expected <recipe>.toml, or a template under files/ or sections/"),
            "{path}: {message}"
        );
    }
}

#[test]
fn recipes_of_the_same_folder_share_a_template() {
    let loaded = project(vec![
        recipe("one", FILE_STEP),
        recipe("two", FILE_STEP),
        file("files/prek.toml", "hooks\n"),
    ])
    .expect("loads");

    assert_eq!(loaded.template("one", "files/prek.toml"), Some("hooks\n"));
    assert_eq!(loaded.template("two", "files/prek.toml"), Some("hooks\n"));
}

#[test]
fn every_problem_of_the_templates_is_reported() {
    let message = rejected(vec![
        recipe("a", FILE_STEP),
        recipe("b", SECTION_STEP),
        file("junk/x", ""),
    ]);

    assert!(message.contains("a.toml"), "{message}");
    assert!(message.contains("b.toml"), "{message}");
    assert!(message.contains("junk/x"), "{message}");
}

fn builtin(files: Vec<SourceFile>) -> Recipes {
    Recipes::from_builtin_files(&files).expect("loads")
}

fn over(built_in: &Recipes, files: Vec<SourceFile>) -> Result<Recipes, CatalogError> {
    Recipes::over(built_in, &files, &[])
}

#[test]
fn a_template_belongs_to_the_built_in_recipes_or_to_the_project_that_has_it() {
    let built_in = builtin(vec![
        recipe("base", FILE_STEP),
        file("files/prek.toml", "built-in\n"),
    ]);

    let all = over(
        &built_in,
        vec![
            recipe("mine", &format!("extends = [\"base\"]\n\n{FILE_STEP}")),
            file("files/prek.toml", "project\n"),
        ],
    )
    .expect("loads");

    assert_eq!(all.template("base", "files/prek.toml"), Some("built-in\n"));
    assert_eq!(all.template("mine", "files/prek.toml"), Some("project\n"));
}

#[test]
fn a_recipe_of_the_project_does_not_find_the_templates_of_the_built_in_recipes() {
    let built_in = builtin(vec![
        recipe("base", FILE_STEP),
        file("files/prek.toml", "built-in\n"),
    ]);

    let error = over(&built_in, vec![recipe("mine", FILE_STEP)]).expect_err("rejected");

    assert!(error.to_string().contains("mine.toml"), "{error}");
    assert!(error.to_string().contains("does not exist"), "{error}");
}

#[test]
fn a_recipe_that_replaces_a_built_in_one_uses_its_own_templates() {
    let built_in = builtin(vec![
        recipe("base", FILE_STEP),
        file("files/prek.toml", "built-in\n"),
    ]);

    let all = Recipes::over(
        &built_in,
        &[
            recipe("base", FILE_STEP),
            file("files/prek.toml", "project\n"),
        ],
        &["base".to_owned()],
    )
    .expect("loads");

    assert_eq!(all.template("base", "files/prek.toml"), Some("project\n"));
}

#[test]
fn a_built_in_recipe_whose_template_is_missing_does_not_load() {
    let error = Recipes::from_builtin_files(&[recipe("base", FILE_STEP)]).expect_err("rejected");

    assert!(error.to_string().contains("does not exist"), "{error}");
}
