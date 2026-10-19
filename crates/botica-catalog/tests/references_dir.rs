use std::fs;
use std::path::{Path, PathBuf};

use botica_catalog::{read_references, References};

fn folder(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("references_dir")
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

const TOPIC: &str = "title = \"T\"\n";

#[test]
fn reads_the_topic_and_the_readme_of_each_folder_with_paths_from_the_folder() {
    let dir = folder("reads");
    write(&dir, "b-topic/topic.toml", TOPIC);
    write(&dir, "b-topic/README.md", "b\n");
    write(&dir, "a-topic/topic.toml", TOPIC);
    write(&dir, "a-topic/README.md", "a\n");
    write(&dir, "a-topic/other.txt", "not read");
    write(&dir, "loose.md", "not a topic");

    let files = read_references(&dir).expect("reads");

    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "a-topic/README.md",
            "a-topic/topic.toml",
            "b-topic/README.md",
            "b-topic/topic.toml"
        ]
    );
    assert!(References::from_files(&files).is_ok());
}

#[test]
fn a_folder_that_does_not_exist_holds_no_topic() {
    let dir = folder("absent");
    let files = read_references(&dir.join("references")).expect("reads");
    assert!(files.is_empty());
}
