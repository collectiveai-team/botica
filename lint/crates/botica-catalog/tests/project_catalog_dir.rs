//! Reading `.botica/rules` from disk.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use botica_catalog::{read_project_files, Catalog};
use common::{family, rule, rule_toml, README};

fn write(dir: &Path, path: &str, text: &str) {
    let target = dir.join(path);
    fs::create_dir_all(target.parent().expect("a parent")).expect("a folder");
    fs::write(target, text).expect("a file");
}

fn fresh(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

const AST_GREP: &str =
    "[[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n";

fn lay_out_acme(dir: &Path) {
    write(dir, "acme/catalog.toml", "code = \"ACME\"\n");
    write(
        dir,
        "acme/structure/family.toml",
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    write(
        dir,
        "acme/structure/no-blobs/rule.toml",
        &rule_toml("acme/no-blobs", "ACMESTRUC001", "", AST_GREP),
    );
    write(dir, "acme/structure/no-blobs/README.md", README);
    write(
        dir,
        "acme/structure/no-blobs/python.yml",
        "id: acme/no-blobs\n",
    );
}

#[test]
fn the_catalogs_of_a_project_are_read_from_its_rules_folder() {
    let dir = fresh("project_catalog_read");
    lay_out_acme(&dir);
    write(
        &dir,
        "acme/structure/no-blobs/fixtures/invalid/a.py",
        "x = 1\n",
    );
    write(&dir, "notes.txt", "not a catalog");

    let files = read_project_files(&dir).expect("the folder is read");

    let built_in = Catalog::from_files(
        &[
            vec![family("structure", "STRUC")],
            rule("structure", "no-utils", "BTSTRUC001"),
        ]
        .concat(),
    )
    .expect("loads");
    let loaded = built_in
        .with_projects(&files)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(loaded.resolve("acme/no-blobs").is_some());
    assert!(
        files
            .iter()
            .all(|f| !f.path.contains("fixtures") && f.path != "notes.txt"),
        "{:?}",
        files.iter().map(|f| &f.path).collect::<Vec<_>>()
    );
}

#[test]
fn a_rules_folder_that_does_not_exist_holds_no_catalog() {
    let dir = fresh("project_catalog_absent").join("no-such-folder");

    assert_eq!(read_project_files(&dir).expect("no error"), []);
}

#[test]
fn a_folder_with_files_and_no_catalog_toml_reaches_the_catalog_to_be_rejected() {
    let dir = fresh("project_catalog_no_toml");
    lay_out_acme(&dir);
    fs::remove_file(dir.join("acme/catalog.toml")).expect("removed");

    let files = read_project_files(&dir).expect("read");

    assert!(files.iter().all(|f| f.path != "acme/catalog.toml"));
    assert!(files.iter().any(|f| f.path == "acme/structure/family.toml"));
}
