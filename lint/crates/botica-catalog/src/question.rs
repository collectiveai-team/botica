use serde::{Deserialize, Serialize};

/// The question a judge answers about a rule, in the form of `@coldtea/abide-schema` 0.0.5: yes or
/// no, one of several options, or a level of a score.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Question {
    Boolean {
        instructions: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        criteria: Option<BooleanCriteria>,
    },
    Choice {
        instructions: String,
        criteria: Choices,
        violating: Vec<String>,
    },
    Score {
        instructions: String,
        /// The levels, from compliant (the first) to the worst.
        criteria: Vec<String>,
        /// The first level that is a violation.
        #[serde(rename = "violatingFrom")]
        violating_from: usize,
    },
}

/// What a yes and a no to a boolean question mean.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanCriteria {
    #[serde(rename = "true", default, skip_serializing_if = "Option::is_none")]
    pub yes: Option<String>,
    #[serde(rename = "false", default, skip_serializing_if = "Option::is_none")]
    pub no: Option<String>,
}

/// The options of a choice question, each with its text, in the order of `rule.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choices(pub Vec<(String, String)>);

impl<'de> Deserialize<'de> for Choices {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Choices;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a table of options, each with its text")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Choices, A::Error> {
                let mut options = Vec::new();
                while let Some(option) = map.next_entry()? {
                    options.push(option);
                }
                Ok(Choices(options))
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

impl Serialize for Choices {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(self.0.iter().map(|(name, text)| (name, text)))
    }
}

impl Question {
    pub fn instructions(&self) -> &str {
        match self {
            Self::Boolean { instructions, .. }
            | Self::Choice { instructions, .. }
            | Self::Score { instructions, .. } => instructions,
        }
    }

    /// The texts of its criteria, in their order.
    pub fn criteria(&self) -> Vec<&str> {
        match self {
            Self::Boolean { criteria, .. } => criteria
                .iter()
                .flat_map(|criteria| [&criteria.yes, &criteria.no])
                .flatten()
                .map(String::as_str)
                .collect(),
            Self::Choice { criteria, .. } => {
                criteria.0.iter().map(|(_, text)| text.as_str()).collect()
            }
            Self::Score { criteria, .. } => criteria.iter().map(String::as_str).collect(),
        }
    }
}

const MAX_INSTRUCTIONS: usize = 2000;
const MAX_CRITERION: usize = 1000;

/// What `@coldtea/abide-schema` 0.0.5 rejects in a question.
pub(crate) fn check_question(question: &Question) -> Vec<String> {
    let mut found = Vec::new();
    let length = question.instructions().chars().count();
    if !(1..=MAX_INSTRUCTIONS).contains(&length) {
        found.push(format!(
            "`instructions` has 1 to {MAX_INSTRUCTIONS} characters"
        ));
    }
    if question
        .criteria()
        .iter()
        .any(|text| text.chars().count() > MAX_CRITERION)
    {
        found.push(format!(
            "a criterion has at most {MAX_CRITERION} characters"
        ));
    }
    match question {
        Question::Boolean { .. } => {}
        Question::Choice {
            criteria,
            violating,
            ..
        } => check_choice(criteria, violating, &mut found),
        Question::Score {
            criteria,
            violating_from,
            ..
        } => {
            if criteria.len() < 2 {
                found.push("a score question needs at least two levels".to_owned());
            }
            if *violating_from < 1 || *violating_from >= criteria.len() {
                found.push(
                    "`violatingFrom` is 1 or more and points at one of the levels".to_owned(),
                );
            }
        }
    }
    found
}

fn check_choice(criteria: &Choices, violating: &[String], found: &mut Vec<String>) {
    let options = &criteria.0;
    if options.len() < 2 {
        found.push("a choice question needs at least two options".to_owned());
    }
    if options.iter().any(|(name, _)| name.is_empty()) {
        found.push("an option of a choice needs a name".to_owned());
    }
    if violating.is_empty() {
        found.push("a choice question needs at least one violating option".to_owned());
    }
    for name in violating {
        if !options.iter().any(|(option, _)| option == name) {
            found.push(format!(
                "violating option `{name}` is not one of the criteria"
            ));
        }
    }
    if violating.len() >= options.len() {
        found.push("at least one option must be compliant".to_owned());
    }
}
