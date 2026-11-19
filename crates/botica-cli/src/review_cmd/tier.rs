use crate::output::println;
use std::process::ExitCode;

use anyhow::{ensure, Context, Result};
use botica_config::{ReviewConfig, Tier};
use botica_core::review::{assess, maximum_tier, Comparison};
use clap::{Args, ValueEnum};
use serde_json::{json, Value};

#[path = "tier_git.rs"]
pub(super) mod git;

#[derive(Args)]
pub(super) struct TierArgs {
    #[arg(long)]
    diff: String,
    #[arg(long, default_value = "HEAD")]
    head: String,
    #[arg(long, value_enum)]
    integration_method: IntegrationMethod,
    #[arg(long, value_enum, default_value = "text")]
    pub(super) format: Format,
}

impl TierArgs {
    pub(super) fn evidence(diff: String, head: String) -> Self {
        Self {
            diff,
            head,
            integration_method: IntegrationMethod::Merge,
            format: Format::Json,
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum IntegrationMethod {
    Merge,
    Squash,
    Rebase,
}

impl IntegrationMethod {
    fn name(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub(super) enum Format {
    Json,
    Text,
}

pub(super) struct TrustedConfig {
    pub source: String,
    pub digest: Option<String>,
    pub table: Option<toml::Table>,
}

fn parse_table(bytes: &[u8]) -> Result<toml::Table> {
    std::str::from_utf8(bytes)
        .context("trusted configuration is not UTF-8")?
        .parse()
        .context("invalid trusted TOML")
}

fn trusted_config(git: &git::Git, identity: &mut TrustedConfig) -> Result<ReviewConfig> {
    let cwd = std::env::current_dir()?;
    let relative = cwd
        .strip_prefix(&git.root)
        .context("current directory outside repository")?;
    for directory in relative.ancestors() {
        let path = |name: &str| -> Result<String> {
            Ok(directory
                .join(name)
                .to_str()
                .context("configuration path is not UTF-8")?
                .to_owned())
        };
        let botica = path("botica.toml")?;
        let pyproject = path("pyproject.toml")?;
        identity.source = botica.clone();
        let standalone = git.blob(&botica)?;
        if let Some(bytes) = &standalone {
            identity.digest = Some(blake3::hash(bytes).to_hex().to_string());
        } else {
            identity.source = pyproject.clone();
        }
        let python = git.blob(&pyproject)?;
        if standalone.is_none() {
            identity.digest = python
                .as_ref()
                .map(|bytes| blake3::hash(bytes).to_hex().to_string());
        }
        let python_table = python
            .as_ref()
            .map(|bytes| parse_table(bytes))
            .transpose()?;
        let python_botica = python_table
            .as_ref()
            .and_then(|table| table.get("tool"))
            .and_then(|tool| tool.get("botica"));
        ensure!(
            standalone.is_none() || python_botica.is_none(),
            "both {botica} and [tool.botica] in {pyproject} configure botica; keep only one"
        );
        let selected = if let Some(bytes) = standalone {
            Some((botica, parse_table(&bytes)?, bytes))
        } else if let Some(table) = python_botica {
            Some((
                pyproject,
                table
                    .as_table()
                    .context("[tool.botica] must be a table")?
                    .clone(),
                python.context("pyproject bytes")?,
            ))
        } else {
            None
        };
        if let Some((source, table, bytes)) = selected {
            identity.source = source;
            identity.digest = Some(blake3::hash(&bytes).to_hex().to_string());
            let review = table
                .get("review")
                .cloned()
                .map(toml::Value::try_into)
                .transpose()
                .context("invalid trusted review settings")?
                .unwrap_or_default();
            let review: ReviewConfig = review;
            review.validate()?;
            identity.table = Some(table);
            return Ok(review);
        }
    }
    identity.source = "defaults".to_owned();
    identity.digest = Some(blake3::hash(b"").to_hex().to_string());
    Ok(ReviewConfig::default())
}

fn incomplete(code: &str, message: impl ToString, comparisons: Vec<Comparison>) -> Value {
    json!({ "status": "incomplete", "reasons": [{"code": code, "message": message.to_string()}], "comparisons": comparisons })
}

fn assessment(comparisons: Vec<Comparison>, mut reasons: Vec<Value>) -> Value {
    reasons.extend(comparisons.iter().filter(|comparison| comparison.bounds.is_none()).flat_map(|comparison| comparison.detector_evidence.iter())
        .filter(|detector| detector.status == "unsupported")
        .map(|detector| json!({"code": "unsupported-detector", "message": format!("{}: {:?}", detector.detail, detector.paths)})));
    if !reasons.is_empty() {
        return json!({"status": "incomplete", "reasons": reasons, "comparisons": comparisons});
    }
    let tier = maximum_tier(comparisons.iter().map(|comparison| comparison.tier));
    json!({ "status": "complete", "tier": tier, "comparisons": comparisons })
}

fn branch(git: &git::Git, config: &ReviewConfig) -> Value {
    if git.shallow {
        return incomplete(
            "missing-history",
            "Fetch the complete history before assessing branch risk",
            vec![],
        );
    }
    let output = match git.output(&["merge-base", "--all", &git.base, &git.head]) {
        Ok(output) => output,
        Err(error) => return incomplete("missing-history", error, vec![]),
    };
    if output.status.code() == Some(1) && output.stdout.is_empty() && output.stderr.is_empty() {
        return incomplete(
            "no-common-ancestor",
            "No common ancestor; fetch missing history if this repository is incomplete",
            vec![],
        );
    }
    if !output.status.success() {
        return incomplete(
            "missing-history",
            String::from_utf8_lossy(&output.stderr),
            vec![],
        );
    }
    let ancestors = match std::str::from_utf8(&output.stdout) {
        Ok(ancestors) if !ancestors.trim().is_empty() => ancestors,
        _ => return incomplete("missing-history", "No readable merge-base evidence", vec![]),
    };
    let mut ancestors: Vec<_> = ancestors.lines().collect();
    ancestors.sort_unstable();
    ancestors.dedup();
    let mut comparisons = Vec::new();
    let mut reasons = Vec::new();
    for ancestor in ancestors {
        match git.changes(ancestor, &git.head) {
            Ok(changes) => comparisons.push(assess(config, ancestor, &git.head, &changes)),
            Err(error) => reasons.push(
                json!({"code": "unreadable-diff", "message": format!("{ancestor}: {error:#}")}),
            ),
        }
    }
    assessment(comparisons, reasons)
}

fn integration(git: &git::Git, config: &ReviewConfig, method: IntegrationMethod) -> Value {
    if !matches!(method, IntegrationMethod::Merge) {
        return incomplete(
            "unsupported-integration-method",
            format!("{} integration is not supported", method.name()),
            vec![],
        );
    }
    if git.shallow {
        return incomplete(
            "missing-history",
            "Fetch the complete history before assessing integration risk",
            vec![],
        );
    }
    let paths = match git.merge_paths() {
        Ok(paths) => paths,
        Err(error) => return incomplete("unreadable-diff", error, vec![]),
    };
    if let Err(error) = git.validate_merge_attributes(&paths) {
        return incomplete("integration-failed", error, vec![]);
    }
    let output = match git.merge_tree() {
        Ok(output) => output,
        Err(error) => return incomplete("integration-failed", error, vec![]),
    };
    if !output.status.success() {
        let code = if output.status.code() == Some(1) {
            "merge-conflict"
        } else {
            "integration-failed"
        };
        return incomplete(
            code,
            format!(
                "Merge result unavailable: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            vec![],
        );
    }
    let tree = match std::str::from_utf8(&output.stdout) {
        Ok(value) => value.trim(),
        Err(error) => return incomplete("integration-failed", error, vec![]),
    };
    match git.changes(&git.base, tree) {
        Ok(changes) => assessment(vec![assess(config, &git.base, tree, &changes)], vec![]),
        Err(error) => incomplete("unreadable-diff", error, vec![]),
    }
}

fn print_text(report: &Value) {
    println!(
        "effective risk: {} ({})",
        report["effective_tier"].as_str().unwrap_or("unavailable"),
        report["status"].as_str().unwrap_or("incomplete")
    );
    println!("inputs: {}", report["inputs"]);
    for name in ["branch", "integration"] {
        let assessment = &report[name];
        println!(
            "{name} risk: {}",
            assessment["tier"].as_str().unwrap_or("incomplete")
        );
        if let Some(reasons) = assessment["reasons"].as_array() {
            for reason in reasons {
                println!("  reason: {reason}");
            }
        }
        if let Some(comparisons) = assessment["comparisons"].as_array() {
            for comparison in comparisons {
                println!(
                    "  {} -> {}: {}",
                    comparison["from"].as_str().unwrap_or(""),
                    comparison["to"].as_str().unwrap_or(""),
                    comparison["tier"].as_str().unwrap_or("")
                );
                for key in [
                    "files",
                    "areas",
                    "signals",
                    "evidence",
                    "signal_evidence",
                    "detector_evidence",
                    "size",
                ] {
                    println!("    {key}: {}", comparison[key]);
                }
            }
        }
    }
}

pub(super) struct Assessment {
    pub git: git::Git,
    pub config: TrustedConfig,
    pub review: Option<ReviewConfig>,
    pub report: Value,
}

pub(super) fn collect(args: &TierArgs) -> Result<Assessment> {
    let git = git::Git::open(&args.diff, &args.head)?;
    let mut config = TrustedConfig {
        source: "unavailable".to_owned(),
        digest: None,
        table: None,
    };
    let mut review = None;
    let (branch, integration) = match trusted_config(&git, &mut config) {
        Ok(settings) => {
            let branch = branch(&git, &settings);
            let integration = integration(&git, &settings, args.integration_method);
            review = Some(settings);
            (branch, integration)
        }
        Err(error) => (
            incomplete("invalid-config", format!("{error:#}"), vec![]),
            incomplete("invalid-config", format!("{error:#}"), vec![]),
        ),
    };
    let done = branch["status"] == "complete" && integration["status"] == "complete";
    let tier = if done {
        Some(maximum_tier([&branch, &integration].into_iter().map(
            |risk| {
                risk["tier"]
                    .as_str()
                    .and_then(Tier::parse)
                    .unwrap_or(Tier::Medium)
            },
        )))
    } else {
        None
    };
    let report = json!({
        "schema_version": 1, "status": if done { "complete" } else { "incomplete" },
        "inputs": {
            "base": &git.base, "head": &git.head, "integration_method": args.integration_method.name(),
            "botica_version": env!("CARGO_PKG_VERSION"),
            "config": {"source": &config.source, "digest": &config.digest}
        },
        "branch": branch, "integration": integration, "effective_tier": tier
    });
    Ok(Assessment {
        git,
        config,
        review,
        report,
    })
}

pub(super) fn run(args: &TierArgs) -> Result<ExitCode> {
    let report = collect(args)?.report;
    match args.format {
        Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        Format::Text => print_text(&report),
    }
    Ok(if report["status"] == "complete" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}
