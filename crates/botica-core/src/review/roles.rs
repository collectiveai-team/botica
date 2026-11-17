//! Trusted role metadata and selection, without filesystem access.

use anyhow::{ensure, Context, Result};
use botica_config::{PathMatcher, ReviewConfig, ReviewRole};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Applies {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub tiers: Option<Vec<String>>,
    #[serde(default)]
    pub signals: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckReference {
    pub file: String,
    pub id: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Covered {
    #[serde(default)]
    pub rules: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<CheckReference>,
    #[serde(default)]
    pub jobs: Vec<CheckReference>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct RoleDefinition {
    pub role: String,
    #[serde(default)]
    pub applies: Applies,
    #[serde(default = "stages")]
    pub stages: Vec<String>,
    #[serde(default = "blocking")]
    pub blocking: String,
    #[serde(default)]
    pub owners: Vec<String>,
    #[serde(default = "max_findings")]
    pub max_findings: u32,
    pub covered: Option<Covered>,
}

fn stages() -> Vec<String> {
    vec!["local".into(), "ci".into()]
}
fn blocking() -> String {
    "P1".into()
}
fn max_findings() -> u32 {
    10
}

/// Parse YAML data without executing repository code.
pub fn yaml(text: &str) -> Result<Value> {
    serde_saphyr::from_str(text).context("invalid YAML")
}

/// Require a root-relative, literal path without traversal.
pub fn relative_path(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty() && !path.contains(['\\', '\0', ':']),
        "invalid repository path"
    );
    ensure!(!path.starts_with('/'), "repository path must be relative");
    ensure!(
        path.split('/').all(|part| !matches!(part, "" | "." | "..")),
        "repository path contains traversal"
    );
    Ok(())
}

impl RoleDefinition {
    /// Validate frontmatter against the expected role folder.
    pub fn parse(expected: &str, text: &str) -> Result<Self> {
        ensure!(text.len() <= 1024 * 1024, "role exceeds 1 MiB");
        let normalized = text.replace("\r\n", "\n");
        let frontmatter = normalized
            .strip_prefix("---\n")
            .context("missing role frontmatter")?
            .split_once("\n---\n")
            .context("unterminated role frontmatter")?
            .0;
        let role: Self = serde_saphyr::from_str(frontmatter).context("invalid role frontmatter")?;
        ensure!(
            role.role == expected && ReviewRole::ALL.iter().any(|r| r.name() == expected),
            "role does not match its folder"
        );
        ensure!(
            !role.stages.is_empty()
                && role
                    .stages
                    .iter()
                    .all(|s| matches!(s.as_str(), "local" | "ci")),
            "invalid stages"
        );
        ensure!(
            matches!(role.blocking.as_str(), "P0" | "P1" | "P2" | "P3"),
            "invalid blocking severity"
        );
        ensure!(role.max_findings > 0, "max-findings must be positive");
        ensure!(
            role.owners
                .iter()
                .all(|owner| ReviewConfig::valid_owner(owner)),
            "invalid role owner"
        );
        ensure!(
            role.applies
                .tiers
                .iter()
                .flatten()
                .all(|t| matches!(t.as_str(), "low" | "medium" | "high")),
            "invalid applies.tiers"
        );
        ensure!(
            role.applies.signals.iter().all(|s| !s.trim().is_empty()),
            "blank applies.signals"
        );
        for path in &role.applies.paths {
            ensure!(
                !path.trim().is_empty()
                    && !path.starts_with(['!', '#'])
                    && !path.chars().any(char::is_control),
                "invalid applies.paths"
            );
            PathMatcher::check(path).map_err(anyhow::Error::msg)?;
        }
        PathMatcher::new(&role.applies.paths).map_err(|errors| anyhow::anyhow!("{errors:?}"))?;
        if let Some(covered) = &role.covered {
            for reference in covered.hooks.iter().chain(&covered.jobs) {
                relative_path(&reference.file)?;
                ensure!(!reference.id.trim().is_empty(), "blank covered check id");
            }
        }
        Ok(role)
    }

    /// Explain selection from the union of all comparison evidence.
    pub fn selection(&self, risk: &Value, stage: &str) -> Result<Value> {
        let mut paths = std::collections::BTreeSet::new();
        let mut signals = std::collections::BTreeSet::new();
        for assessment in ["branch", "integration"] {
            for comparison in risk[assessment]["comparisons"]
                .as_array()
                .into_iter()
                .flatten()
            {
                paths.extend(
                    comparison["files"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str),
                );
                signals.extend(
                    comparison["signals"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str),
                );
            }
        }
        let matcher = PathMatcher::new(&self.applies.paths)
            .map_err(|errors| anyhow::anyhow!("{errors:?}"))?;
        let matching_paths: Vec<_> = paths
            .into_iter()
            .filter(|path| self.applies.paths.is_empty() || matcher.is_match(path))
            .collect();
        let matching_signals: Vec<_> = signals
            .into_iter()
            .filter(|signal| self.applies.signals.iter().any(|s| s == signal))
            .collect();
        let stage_matches = self.stages.iter().any(|s| s == stage);
        let paths_match = self.applies.paths.is_empty() || !matching_paths.is_empty();
        let signals_match = self.applies.signals.is_empty() || !matching_signals.is_empty();
        let tier = risk["effective_tier"].as_str();
        let tier_matches = tier.map(|tier| {
            self.applies
                .tiers
                .as_ref()
                .is_none_or(|tiers| tiers.iter().any(|t| t == tier))
        });
        let selected = if risk["status"] == "complete" {
            tier_matches.map(|tier| stage_matches && paths_match && signals_match && tier)
        } else {
            None
        };
        Ok(
            json!({"selected": selected, "stage_matches": stage_matches, "paths_match": paths_match,
            "matching_paths": matching_paths, "signals_match": signals_match, "matching_signals": matching_signals,
            "tier_matches": tier_matches, "effective_tier": tier}),
        )
    }
}
