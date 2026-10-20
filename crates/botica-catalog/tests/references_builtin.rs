//! The topics that the binary carries, and how the topics of a project go with them.

mod common;

use botica_catalog::{CatalogError, Origin, References, SourceFile};
use common::topics::{load, topic, EXTERNAL, INTERNAL, PAGE};

fn builtin(parts: Vec<Vec<SourceFile>>) -> Result<References, CatalogError> {
    References::from_builtin_files(&parts.concat())
}

/// The built-in topics `logging` and `testing`, which point at external files, and the topic
/// `mine` of a project.
fn world() -> (References, Vec<SourceFile>) {
    let builtin = builtin(vec![topic("logging", EXTERNAL), topic("testing", PAGE)]).expect("loads");
    (builtin, topic("mine", INTERNAL))
}

fn over(
    builtin: &References,
    project: &[SourceFile],
    replace: &[&str],
) -> Result<References, CatalogError> {
    let replace: Vec<String> = replace.iter().map(|id| (*id).to_owned()).collect();
    References::over(builtin, project, &replace)
}

fn ids(topics: &References) -> Vec<&str> {
    topics.topics().iter().map(|t| t.id.as_str()).collect()
}

#[test]
fn a_topic_knows_whether_it_is_built_in_or_of_the_project() {
    let built_in = builtin(vec![topic("logging", EXTERNAL)]).expect("loads");
    assert_eq!(
        built_in.topic("logging").expect("topic").origin,
        Origin::BuiltIn
    );

    let project = load(vec![topic("logging", EXTERNAL)]).expect("loads");
    assert_eq!(
        project.topic("logging").expect("topic").origin,
        Origin::Project
    );
}

#[test]
fn a_built_in_topic_may_point_at_pages_and_at_external_files() {
    builtin(vec![topic("a", EXTERNAL), topic("b", PAGE)]).expect("loads");
}

#[test]
fn a_built_in_topic_cannot_point_at_a_file_of_the_project() {
    let message = builtin(vec![topic("logging", INTERNAL)])
        .expect_err("rejected")
        .to_string();

    assert!(message.contains("logging/topic.toml"), "{message}");
    assert!(
        message.contains("a built-in topic points at its own code, external files and pages only"),
        "{message}"
    );
    assert!(message.contains("references[0]"), "{message}");
}

#[test]
fn the_topics_of_a_project_go_with_the_built_in_ones_in_the_order_of_their_ids() {
    let (builtin, project) = world();

    let all = over(&builtin, &project, &[]).expect("loads");

    assert_eq!(ids(&all), ["logging", "mine", "testing"]);
    assert_eq!(all.topic("logging").expect("topic").origin, Origin::BuiltIn);
    assert_eq!(all.topic("mine").expect("topic").origin, Origin::Project);
}

#[test]
fn a_project_with_no_topics_has_the_built_in_ones() {
    let (builtin, _) = world();

    let all = over(&builtin, &[], &[]).expect("loads");

    assert_eq!(ids(&all), ["logging", "testing"]);
}

#[test]
fn a_project_topic_with_the_id_of_a_built_in_one_is_an_error() {
    let (builtin, _) = world();

    let message = over(&builtin, &topic("logging", INTERNAL), &[])
        .expect_err("rejected")
        .to_string();

    assert!(message.contains("logging/topic.toml"), "{message}");
    assert!(
        message.contains("the id `logging` is the one of a built-in topic"),
        "{message}"
    );
    assert!(
        message.contains("`replace` in the `[references]` table"),
        "{message}"
    );
}

#[test]
fn a_topic_that_the_configuration_replaces_is_the_one_of_the_project() {
    let (builtin, _) = world();

    let all = over(&builtin, &topic("logging", INTERNAL), &["logging"]).expect("loads");

    assert_eq!(ids(&all), ["logging", "testing"]);
    let replaced = all.topic("logging").expect("topic");
    assert_eq!(replaced.origin, Origin::Project);
    assert!(
        !replaced.references[0].is_external(),
        "the topic of the project"
    );
    assert_eq!(all.topic("testing").expect("topic").origin, Origin::BuiltIn);
}

#[test]
fn replace_names_a_built_in_topic_that_the_project_defines() {
    let (builtin, project) = world();

    let message = over(&builtin, &project, &["ghost"])
        .expect_err("rejected")
        .to_string();
    assert!(message.contains("replace"), "{message}");
    assert!(
        message.contains("`ghost` is not a built-in topic"),
        "{message}"
    );

    let message = over(&builtin, &project, &["logging"])
        .expect_err("rejected")
        .to_string();
    assert!(message.contains("replace"), "{message}");
    assert!(
        message
            .contains("`logging` is a built-in topic, and `references/` has no topic with that id"),
        "{message}"
    );
}

#[test]
fn a_project_that_does_not_load_reports_its_own_problems() {
    let (builtin, _) = world();

    let message = over(&builtin, &topic("mine", "title = 1\n"), &[])
        .expect_err("rejected")
        .to_string();

    assert!(message.contains("mine/topic.toml"), "{message}");
}

#[test]
fn the_embedded_topics_load_and_every_one_is_built_in() {
    let embedded = References::embedded().expect("the embedded topics load");

    assert!(!embedded.topics().is_empty());
    for topic in embedded.topics() {
        assert_eq!(topic.origin, Origin::BuiltIn, "{}", topic.id);
        assert!(!topic.references.is_empty(), "{}", topic.id);
    }
}
