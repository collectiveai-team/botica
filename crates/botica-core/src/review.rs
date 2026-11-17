//! Pure risk assessment of collected Git evidence.

use std::collections::BTreeSet;

use botica_config::{PathMatcher, ReviewConfig, Tier};
use serde::Serialize;

mod python;
mod signals;

/// One changed file, with both paths of a rename and raw text size when available.
#[derive(Debug, Clone)]
pub struct ReviewChange {
    pub paths: Vec<String>,
    pub added: Option<u64>,
    pub deleted: Option<u64>,
    pub kind: String,
    pub status: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AreaEvidence {
    pub location: String,
    pub area: String,
    pub tier: String,
    pub paths: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FileEvidence {
    pub paths: Vec<String>,
    pub kind: String,
    pub status: String,
    pub added: Option<u64>,
    pub deleted: Option<u64>,
    pub generated: bool,
    pub matches: Vec<AreaEvidence>,
}

#[derive(Debug, Serialize)]
pub struct SizeEvidence {
    pub changed_lines: u64,
    pub non_text_files: Vec<String>,
    pub informational_only: bool,
}

#[derive(Debug, Serialize)]
pub struct SignalEvidence {
    pub name: String,
    pub paths: Vec<String>,
    pub detector: String,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct DetectorEvidence {
    pub detector: String,
    pub status: String,
    pub paths: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub from: String,
    pub to: String,
    pub tier: String,
    pub files: Vec<String>,
    pub areas: Vec<String>,
    pub signals: Vec<String>,
    pub signal_evidence: Vec<SignalEvidence>,
    pub detector_evidence: Vec<DetectorEvidence>,
    pub evidence: Vec<FileEvidence>,
    pub size: SizeEvidence,
}

/// Documentation filenames supported by the path-only documentation detector.
pub fn is_documentation(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if [
        "requirements",
        "constraints",
        "packages",
        "manifest",
        "lockfile",
    ]
    .iter()
    .any(|name| lower.contains(name))
    {
        return false;
    }
    if lower.ends_with(".txt") {
        return matches!(
            lower.rsplit('/').next(),
            Some("readme.txt" | "license.txt" | "notice.txt" | "changelog.txt")
        );
    }
    [".md", ".rst", ".adoc"]
        .iter()
        .any(|extension| lower.ends_with(extension))
}

fn known_policy_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    matches!(
        name.as_str(),
        "botica.toml" | ".gitattributes" | "codeowners"
    )
}

fn matches(patterns: &[String], path: &str) -> bool {
    PathMatcher::new(patterns).is_ok_and(|matcher| matcher.is_match(path))
}

fn areas(config: &ReviewConfig, paths: &[String]) -> Vec<AreaEvidence> {
    let mut evidence = Vec::new();
    for path in paths {
        let forced = config.is_forced_platform(path);
        let mut matched = false;
        for (index, area) in config.areas.iter().enumerate() {
            if matches(&area.paths, path) {
                matched = true;
                evidence.push(AreaEvidence {
                    location: format!("review.areas[{index}]"),
                    area: area.name.name().to_owned(),
                    tier: area.effective_tier().name().to_owned(),
                    paths: vec![path.clone()],
                });
            }
        }
        if forced {
            let tier = config
                .areas
                .iter()
                .filter(|area| area.name == botica_config::AreaName::Platform)
                .map(|area| area.effective_tier())
                .max()
                .unwrap_or(Tier::Medium);
            evidence.push(AreaEvidence {
                location: "forced-platform".to_owned(),
                area: "platform".to_owned(),
                tier: tier.name().to_owned(),
                paths: vec![path.clone()],
            });
        } else if !matched {
            evidence.push(AreaEvidence {
                location: "default-core".to_owned(),
                area: "core".to_owned(),
                tier: "medium".to_owned(),
                paths: vec![path.clone()],
            });
        }
    }
    evidence
}

fn rank(tier: &str) -> u8 {
    match tier {
        "high" => 2,
        "medium" => 1,
        _ => 0,
    }
}

/// Return the maximum tier; an empty change has the core floor of medium.
pub fn maximum_tier<'a>(tiers: impl Iterator<Item = &'a str>) -> &'a str {
    tiers.max_by_key(|tier| rank(tier)).unwrap_or("medium")
}

/// Classify paths using the destination's validated review settings.
pub fn assess(config: &ReviewConfig, from: &str, to: &str, changes: &[ReviewChange]) -> Comparison {
    let mut files = BTreeSet::new();
    let mut touched = BTreeSet::new();
    let mut evidence = Vec::new();
    let mut size = SizeEvidence {
        changed_lines: 0,
        non_text_files: Vec::new(),
        informational_only: true,
    };
    for change in changes {
        files.extend(change.paths.iter().cloned());
        let matched = areas(config, &change.paths);
        touched.extend(matched.iter().map(|entry| entry.area.clone()));
        let protected = matched
            .iter()
            .any(|entry| matches!(entry.area.as_str(), "platform" | "critical"));
        let generated = !protected
            && change
                .paths
                .iter()
                .all(|path| matches(&config.generated, path));
        if !generated {
            match (change.added, change.deleted) {
                (Some(added), Some(deleted)) => {
                    size.changed_lines = size
                        .changed_lines
                        .saturating_add(added)
                        .saturating_add(deleted)
                }
                _ => size.non_text_files.extend(change.paths.iter().cloned()),
            }
        }
        evidence.push(FileEvidence {
            paths: change.paths.clone(),
            kind: change.kind.clone(),
            status: change.status.clone(),
            added: change.added,
            deleted: change.deleted,
            generated,
            matches: matched,
        });
    }
    let mut tier = maximum_tier(
        evidence
            .iter()
            .flat_map(|file| file.matches.iter().map(|area| area.tier.as_str())),
    )
    .to_owned();
    let docs_only = !changes.is_empty()
        && changes.iter().all(|change| {
            change.kind == "text" && change.paths.iter().all(|path| is_documentation(path))
        });
    let mut signal_evidence = if docs_only {
        vec![SignalEvidence {
            name: "docs-only".to_owned(),
            paths: files.iter().cloned().collect(),
            detector: "documentation-extensions-v1".to_owned(),
            detail: "Every changed path has a documentation extension and a text diff".to_owned(),
        }]
    } else {
        Vec::new()
    };
    let (detector_evidence, facts) = signals::detect(changes);
    signal_evidence.extend(facts);
    if signal_evidence
        .iter()
        .any(|signal| matches!(signal.name.as_str(), "new-dependency" | "new-endpoint"))
        && tier == "low"
    {
        tier = "medium".to_owned();
    }
    let names: BTreeSet<_> = signal_evidence
        .iter()
        .map(|signal| signal.name.clone())
        .collect();
    Comparison {
        from: from.to_owned(),
        to: to.to_owned(),
        tier,
        files: files.into_iter().collect(),
        areas: touched.into_iter().collect(),
        signals: names.into_iter().collect(),
        signal_evidence,
        detector_evidence,
        evidence,
        size,
    }
}
