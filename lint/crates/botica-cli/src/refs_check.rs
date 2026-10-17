use std::process::ExitCode;

use botica_catalog::{References, Topic, REFERENCES_DIR, TOPIC_FILE};

use crate::refs_collection::{target, Collection};
use crate::refs_standards::Problem;

/// Checks what `references/` holds and what the standards and the rules of the project cite. The
/// rules of the built-in catalog cite topics of the checkout of botica, so they are left to the
/// tests of that checkout.
pub fn run(collection: &Collection) -> ExitCode {
    let mut problems = collection.standard_problems.clone();
    let topics = match &collection.topics {
        Ok(topics) => {
            problems.extend(topic_problems(collection, topics));
            problems.extend(citation_problems(collection, topics));
            Some(topics)
        }
        Err(error) => {
            problems.extend(error.issues.iter().map(|issue| Problem {
                path: format!("{REFERENCES_DIR}/{}", issue.path),
                message: issue.message.clone(),
            }));
            None
        }
    };
    if problems.is_empty() {
        let topics = topics.map_or(&[][..], References::topics);
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

fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

fn topic_problems(collection: &Collection, topics: &References) -> Vec<Problem> {
    let mut problems = Vec::new();
    for topic in topics.topics() {
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
    let rules = collection
        .project
        .catalog
        .rules()
        .iter()
        .filter(|rule| rule.catalog.is_some())
        .flat_map(|rule| {
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
