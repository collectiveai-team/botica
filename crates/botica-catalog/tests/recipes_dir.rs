use std::fs;
use std::path::{Path, PathBuf};

use botica_catalog::{read_recipes, Recipes};

fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("recipes_dir")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the folder");
    dir
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().expect("a parent")).expect("create parents");
    fs::write(path, text).expect("write the file");
}

#[test]
fn reads_the_toml_files_of_the_folder_with_paths_from_the_folder() {
    let dir = folder("reads");
    write(&dir, "b-recipe.toml", "rules = [\"b\"]\n");
    write(&dir, "a-recipe.toml", "rules = [\"a\"]\n");
    write(&dir, "README.md", "not a recipe");
    write(&dir, "notes.txt", "not a recipe");
    write(&dir, "nested/c-recipe.toml", "rules = [\"c\"]\n");

    let files = read_recipes(&dir).expect("reads");

    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["a-recipe.toml", "b-recipe.toml"]);
    assert_eq!(files[0].text, "rules = [\"a\"]\n");
    assert!(Recipes::from_files(&files).is_ok());
}

#[test]
fn a_folder_that_does_not_exist_holds_no_recipe() {
    let dir = folder("absent");

    let files = read_recipes(&dir.join("recipes")).expect("reads");

    assert!(files.is_empty());
}

#[test]
fn reads_the_templates_under_files_and_sections_at_any_depth() {
    let dir = folder("templates");
    write(
        &dir,
        "base.toml",
        "[[files]]\npath = \"ci.yml\"\ntemplate = \"files/github/ci.yml\"\n\n\
         [[sections]]\npath = \"AGENTS.md\"\nmarker = \"## M\"\ntemplate = \"sections/m.md\"\n",
    );
    write(&dir, "files/github/ci.yml", "on: push\n");
    write(&dir, "files/prek.toml", "hooks\n");
    write(&dir, "sections/m.md", "## M\n");
    write(&dir, "other/x.md", "not a template");
    write(&dir, "files-extra/x.md", "not a template");

    let files = read_recipes(&dir).expect("reads");

    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "base.toml",
            "files/github/ci.yml",
            "files/prek.toml",
            "sections/m.md"
        ]
    );
    let recipes = Recipes::from_files(&files).expect("loads");
    assert_eq!(
        recipes.template("base", "files/github/ci.yml"),
        Some("on: push\n")
    );
    assert_eq!(recipes.template("base", "sections/m.md"), Some("## M\n"));
}
