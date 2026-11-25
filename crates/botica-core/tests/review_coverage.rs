//! The coverage block of a role, built from the hook and job sources that a reader returns.

use std::collections::BTreeMap;

use anyhow::Result;
use botica_core::review::coverage::{check_references, current_block, render};
use botica_core::review::roles::{CheckReference, Covered};

const PREK: &str = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"botica\"\n";
const CI: &str = "jobs:\n  test:\n    runs-on: ubuntu-latest\n";

fn reference(file: &str, id: &str) -> CheckReference {
    CheckReference {
        file: file.into(),
        id: id.into(),
    }
}

fn covered() -> Covered {
    Covered {
        rules: Vec::new(),
        hooks: vec![reference("prek.toml", "botica")],
        jobs: vec![reference(".github/workflows/ci.yml", "test")],
    }
}

fn files() -> BTreeMap<&'static str, &'static str> {
    BTreeMap::from([("prek.toml", PREK), (".github/workflows/ci.yml", CI)])
}

fn block(covered: &Covered, files: &BTreeMap<&str, &str>) -> Result<String> {
    let mut references = check_references(covered, |path| {
        Ok(files.get(path).map(|text| text.as_bytes().to_vec()))
    })?;
    render(&mut references)
}

#[test]
fn renders_sorted_lines_with_the_digest_of_each_whole_source() {
    let expected = format!(
        "<!-- botica:covered -->\n\
         - hook \"botica\" in \"prek.toml\": blake3 {}\n\
         - job \"test\" in \".github/workflows/ci.yml\": blake3 {}\n\
         <!-- /botica:covered -->",
        blake3::hash(PREK.as_bytes()).to_hex(),
        blake3::hash(CI.as_bytes()).to_hex()
    );
    assert_eq!(block(&covered(), &files()).unwrap(), expected);
}

#[test]
fn an_edit_of_a_source_changes_the_block() {
    let before = block(&covered(), &files()).unwrap();
    let mut edited = files();
    edited.insert(
        ".github/workflows/ci.yml",
        "jobs:\n  test:\n    runs-on: macos-latest\n",
    );
    assert_ne!(block(&covered(), &edited).unwrap(), before);
}

#[test]
fn a_missing_source_or_check_fails() {
    let mut missing = files();
    missing.remove("prek.toml");
    let error = block(&covered(), &missing).unwrap_err();
    assert!(format!("{error:#}").contains("missing trusted coverage source prek.toml"));

    let mut gone = covered();
    gone.jobs = vec![reference(".github/workflows/ci.yml", "lint")];
    let error = block(&gone, &files()).unwrap_err();
    assert!(format!("{error:#}").contains("missing trusted job lint"));
}

#[test]
fn a_duplicate_declaration_fails() {
    let mut twice = covered();
    twice.hooks.push(reference("prek.toml", "botica"));
    let error = block(&twice, &files()).unwrap_err();
    assert!(format!("{error:#}").contains("duplicate coverage declarations"));
}

#[test]
fn current_block_reads_crlf_and_rejects_bad_markers() {
    let text = "intro\r\n<!-- botica:covered -->\r\n- x\r\n<!-- /botica:covered -->\r\nend";
    assert_eq!(
        current_block(text).unwrap().as_deref(),
        Some("<!-- botica:covered -->\n- x\n<!-- /botica:covered -->")
    );
    assert_eq!(current_block("no markers").unwrap(), None);
    assert!(current_block("<!-- /botica:covered --> <!-- botica:covered -->").is_err());
    assert!(current_block("<!-- botica:covered --><!-- botica:covered -->").is_err());
}
