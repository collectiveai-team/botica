//! The report of `abide check --json` (`@coldtea/abide` 0.0.11) as findings: one section per
//! phase, with a verdict per rule. A verdict names no line, so each finding is about a file, or
//! about the whole change when the verdict of the `turn` phase names no file.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::Value;

use super::Found;

#[derive(Deserialize)]
struct Report {
    sections: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    phase: Phase,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    verdicts: Vec<Verdict>,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Phase {
    Edit,
    Turn,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Verdict {
    rule_id: String,
    probability: f64,
    band: Band,
    answer: Option<String>,
    file: Option<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Band {
    Clear,
    Flag,
    Act,
}

impl Band {
    fn name(self) -> &'static str {
        match self {
            Band::Clear => "clear",
            Band::Flag => "flag",
            Band::Act => "act",
        }
    }
}

/// The verdicts of the report in the bands `act` and `flag`; `flag` is a finding the judge is
/// unsure of. A verdict in the band `clear` is no finding.
pub(super) fn findings(document: Value) -> Result<Vec<Found>> {
    let report: Report = serde_json::from_value(document)
        .context("the judged findings are not a valid report of `abide check --json`")?;
    let mut found = Vec::new();
    for section in report.sections {
        let file = match section.phase {
            Phase::Edit => section.files.into_iter().next(),
            Phase::Turn => None,
        };
        for verdict in section.verdicts {
            if verdict.band == Band::Clear {
                continue;
            }
            let judged = match verdict.band {
                Band::Flag => "abide may see the change break it",
                _ => "abide sees the change break it",
            };
            let mut note = format!(
                "{judged}: {}, probability {:.2}",
                verdict.band.name(),
                verdict.probability
            );
            if let Some(answer) = &verdict.answer {
                note.push_str(&format!(", answer {answer}"));
            }
            found.push(Found {
                rule_id: verdict.rule_id,
                message: None,
                note: Some(note),
                path: verdict.file.or_else(|| file.clone()).unwrap_or_default(),
                region: None,
                unsure: verdict.band == Band::Flag,
                probability: Some(verdict.probability),
            });
        }
    }
    Ok(found)
}
