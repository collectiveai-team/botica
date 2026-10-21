//! `botica refs export` prints every topic as JSON, with the text that each reference shows.

mod common;

use std::path::{Path, PathBuf};

use common::builtin::{counts, fill_cache};
use common::git_remote::{refs_run, Origin, Setup};
use common::refs::{add_topic, failed, ok, topic_toml, PAGE, SOURCE};
use common::{project_in, write};
use serde_json::Value;

const REPO: &str = "example.invalid/acme/app";

fn cache(name: &str) -> PathBuf {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("refs-export-cache")
        .join(name);
    let _ = std::fs::remove_dir_all(&cache);
    fill_cache(&cache);
    cache
}

fn export(dir: &Path, cache: &Path, maps: Vec<(&str, &Origin)>) -> std::process::Output {
    let cache = cache.display().to_string();
    refs_run(
        dir,
        &["export", "--cache-dir", &cache],
        &Setup {
            maps,
            ..Setup::default()
        },
    )
}

fn topics_of(output: &std::process::Output) -> Vec<Value> {
    let json: Value = serde_json::from_str(&ok(output)).expect("valid JSON");
    json["topics"].as_array().expect("a list of topics").clone()
}

fn find<'a>(topics: &'a [Value], id: &str) -> &'a Value {
    topics
        .iter()
        .find(|topic| topic["id"] == id)
        .unwrap_or_else(|| panic!("no topic {id}"))
}

#[test]
fn a_topic_of_the_project_comes_with_its_readme_and_the_text_of_each_reference() {
    let dir = project_in("refs-export", "project", None);
    write(&dir, "src/app.py", SOURCE);
    add_topic(&dir, "demo", &topic_toml(PAGE));

    let topics = topics_of(&export(&dir, &cache("project"), vec![]));

    let demo = find(&topics, "demo");
    assert_eq!(demo["title"], "Demo topic");
    assert_eq!(demo["origin"], "project");
    assert_eq!(demo["readme"], "Read `get_logger`.\n");
    let [file, page] = demo["references"]
        .as_array()
        .expect("references")
        .as_slice()
    else {
        panic!("two references expected: {demo}");
    };
    assert_eq!(file["path"], "src/app.py");
    assert_eq!(file["lines"], "3-4");
    assert_eq!(file["anchor"], "def get_logger");
    assert_eq!(file["note"], "the setup");
    assert_eq!(file["text"], "def get_logger():\n    pass\n");
    assert_eq!(file["repo"], Value::Null);
    assert_eq!(page["url"], "https://example.invalid/docs#setup");
    assert_eq!(page["note"], "the documentation of the library");
    assert_eq!(page["text"], Value::Null);
}

#[test]
fn the_text_of_an_external_reference_is_the_lines_of_its_commit() {
    let origin = Origin::new("refs-export-origin");
    let first = origin.commit(&[("src/a.py", b"one\ntwo\nthree\n")]);
    origin.commit(&[("src/a.py", b"changed\n")]);
    let dir = project_in("refs-export", "external", None);
    add_topic(
        &dir,
        "ext",
        &format!(
            "title = \"E\"\n\n[[references]]\nrepo = \"{REPO}\"\nrev = \"{first}\"\nlicense = \"MIT\"\n\
             path = \"src/a.py\"\nlines = \"2-3\"\nnote = \"n\"\n"
        ),
    );

    let topics = topics_of(&export(&dir, &cache("external"), vec![(REPO, &origin)]));

    let reference = &find(&topics, "ext")["references"][0];
    assert_eq!(reference["text"], "two\nthree\n");
    assert_eq!(reference["repo"], REPO);
    assert_eq!(reference["rev"], first);
    assert_eq!(reference["license"], "MIT");
}

#[test]
fn the_built_in_topics_are_in_the_export_with_the_text_of_their_references() {
    let dir = project_in("refs-export", "built-in", None);

    let topics = topics_of(&export(&dir, &cache("built-in"), vec![]));

    let (count, references) = counts();
    assert_eq!(topics.len(), count);
    let logging = find(&topics, "structured-logging");
    assert_eq!(logging["origin"], "built-in");
    let files: Vec<&Value> = topics
        .iter()
        .flat_map(|topic| topic["references"].as_array().expect("references"))
        .collect();
    assert_eq!(files.len(), references);
    assert!(logging["references"][0]["text"]
        .as_str()
        .expect("text")
        .contains("def _configure"),);
}

#[test]
fn a_reference_that_cannot_be_read_stops_the_export_and_names_it() {
    let dir = project_in("refs-export", "unreachable", None);
    add_topic(
        &dir,
        "ext",
        &format!(
            "title = \"E\"\n\n[[references]]\nrepo = \"{REPO}\"\nrev = \"{}\"\nlicense = \"MIT\"\n\
             path = \"a.py\"\nnote = \"n\"\n",
            "0".repeat(40)
        ),
    );

    let output = export(&dir, &cache("unreachable"), vec![]);

    let text = failed(&output, 2);
    assert!(text.contains("topic `ext`"), "{text}");
    assert!(text.contains(&format!("cannot fetch {REPO}@")), "{text}");
    assert!(output.stdout.is_empty(), "nothing is printed half done");
}
