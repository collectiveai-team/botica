use std::process::ExitCode;

use botica_catalog::{Origin, References, Topic, REFERENCES_DIR, TOPIC_FILE};

use crate::refs_collection::{target, Collection};
use crate::refs_standards::Problem;

/// Checks that every rule and standard cites a topic that exists, and that the references of the
/// topics of the project resolve. The built-in topics are not fetched unless `all` is set: their
/// references point at botica, and botica checks them.
pub fn run(collection: &Collection, all: bool) -> ExitCode {
    let mut problems = collection.standard_problems.clone();
    let topics = match &collection.topics {
        Ok(topics) => {
            let checked = checked(topics, all);
            problems.extend(topic_problems(collection, &checked));
            problems.extend(citation_problems(collection, topics));
            checked
        }
        Err(error) => {
            problems.extend(error.issues.iter().map(|issue| Problem {
                path: issue_path(&issue.path),
                message: issue.message.clone(),
            }));
            Vec::new()
        }
    };
    if problems.is_empty() {
        let references: usize = topics.iter().map(|topic| topic.references.len()).sum();
        println!(
            "{} and {} resolve",
            plural(topics.len(), "topic"),
            plural(references, "reference")
        );
        return ExitCode::SUCCESS;
    }
    for problem in &problems {
        eprintln!("{problem}");
    }
    eprintln!("{}", plural(problems.len(), "problem"));
    ExitCode::from(1)
}

/// A problem of a file of `references/` is named from the root of the project; one of the
/// configuration (`[references] replace`) is named as it is.
fn issue_path(path: &str) -> String {
    if path.starts_with('[') {
        path.to_owned()
    } else {
        format!("{REFERENCES_DIR}/{path}")
    }
}

fn checked(topics: &References, all: bool) -> Vec<&Topic> {
    topics
        .topics()
        .iter()
        .filter(|topic| all || topic.origin == Origin::Project)
        .collect()
}

fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn topic_problems(collection: &Collection, topics: &[&Topic]) -> Vec<Problem> {
    let mut problems = Vec::new();
    for topic in topics {
        for (position, reference) in topic.references.iter().enumerate() {
            let outcome = match &reference.path {
                None => Ok(()),
                Some(path) => collection
                    .file_text(reference, path)
                    .and_then(|text| reference.check(&text)),
            };
            if let Err(message) = outcome {
                problems.push(topic_problem(topic, position, &message));
            }
        }
    }
    problems
}

fn topic_problem(topic: &Topic, position: usize, message: &str) -> Problem {
    Problem {
        path: format!("{REFERENCES_DIR}/{}/{TOPIC_FILE}", topic.id),
        message: format!(
            "references[{position}] ({}): {message}",
            target(&topic.references[position])
        ),
    }
}

fn citation_problems(collection: &Collection, topics: &References) -> Vec<Problem> {
    let missing = |cited: &[String]| -> Vec<String> {
        cited
            .iter()
            .filter(|id| topics.topic(id).is_none())
            .map(|id| format!("cites the topic `{id}`, which is not in {REFERENCES_DIR}/"))
            .collect()
    };
    let rules = collection.project.catalog.rules().iter().flat_map(|rule| {
        missing(&rule.references)
            .into_iter()
            .map(|message| Problem {
                path: format!("rule {}", rule.id),
                message,
            })
    });
    let standards = collection.standards.iter().flat_map(|standard| {
        missing(&standard.references)
            .into_iter()
            .map(|message| Problem {
                path: standard.path.clone(),
                message,
            })
    });
    rules.chain(standards).collect()
}
