use crate::issue::{issue, CatalogError};
use crate::load::SourceFile;
use crate::references::{References, Topic, TOPIC_FILE};

/// The path of the problems with `replace`: they are in the configuration, not in a file of
/// `references/`.
const REPLACE_KEY: &str = "[references] replace";

impl References {
    /// The topics of a project together with the built-in ones, in the order of their ids. A topic
    /// of the project with the id of a built-in topic is an error unless `replace` names the id;
    /// then it takes the place of the built-in topic.
    pub fn over(
        builtin: &References,
        project: &[SourceFile],
        replace: &[String],
    ) -> Result<Self, CatalogError> {
        let project = Self::from_files(project)?;
        let mut issues = Vec::new();
        for id in replace {
            match (builtin.topic(id), project.topic(id)) {
                (None, _) => issues.push(issue(
                    REPLACE_KEY,
                    format!("`{id}` is not a built-in topic"),
                )),
                (Some(_), None) => issues.push(issue(
                    REPLACE_KEY,
                    format!(
                        "`{id}` is a built-in topic, and `references/` has no topic with that id"
                    ),
                )),
                (Some(_), Some(_)) => {}
            }
        }
        for topic in &project.topics {
            if builtin.topic(&topic.id).is_some() && !replace.contains(&topic.id) {
                issues.push(issue(
                    format!("{}/{TOPIC_FILE}", topic.id),
                    format!(
                        "the id `{}` is the one of a built-in topic; to replace that topic, name \
                         it in `replace` in the `[references]` table of the configuration",
                        topic.id
                    ),
                ));
            }
        }
        if !issues.is_empty() {
            return Err(CatalogError { issues });
        }
        let mut topics: Vec<Topic> = builtin
            .topics
            .iter()
            .filter(|topic| !replace.contains(&topic.id))
            .cloned()
            .chain(project.topics)
            .collect();
        topics.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(Self { topics })
    }
}
