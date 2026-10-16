//! The topics of `references/`: a `topic.toml` that lists references to code, and a `README.md`.

use botica_catalog::{Lines, Place, SourceFile, Topics};

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

const README: &str = "What to read in each reference.\n";

fn load(topic_toml: &str) -> Topics {
    Topics::from_files(&[
        file("logging/topic.toml", topic_toml),
        file("logging/README.md", README),
    ])
    .expect("the topic loads")
}

#[test]
fn an_internal_reference_has_a_path_lines_an_anchor_and_a_note() {
    let topics = load(
        "title = \"Logging\"\ndocs = [\"https://example.org/log\"]\n\n[[references]]\n\
         path = \"core/logger.py\"\nlines = \"33-75\"\nanchor = \"def get_logger\"\n\
         note = \"the setup\"\n",
    );

    let topic = topics.get("logging").expect("the topic");

    assert_eq!(topic.title, "Logging");
    assert_eq!(topic.docs, ["https://example.org/log"]);
    assert_eq!(topic.readme, README);
    let reference = &topic.references[0];
    assert_eq!(reference.place, Place::Internal);
    assert_eq!(reference.path, "core/logger.py");
    assert_eq!(reference.lines, Some(Lines { start: 33, end: 75 }));
    assert_eq!(reference.anchor.as_deref(), Some("def get_logger"));
    assert_eq!(reference.note, "the setup");
}
