use crate::output::println;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;
use std::time::SystemTime;

use anyhow::{ensure, Context, Result};
use botica_core::fingerprint::{FingerprintKey, KEY_VARIABLE};
use botica_core::history::{self, FindingLine, RuleEntry, RunLine};
use clap::Args;
use serde_json::{json, Value};

use super::{consumers, tier};

#[derive(Args)]
pub(super) struct EvidenceArgs {
    #[command(flatten)]
    pub(super) plan: consumers::ConsumerArgs,
    #[arg(long)]
    results: std::path::PathBuf,
    #[arg(long)]
    record: bool,
}

pub(super) fn read_json(path: &Path) -> Result<Value> {
    let metadata = std::fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "evidence must be a regular file"
    );
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 2 * 1024 * 1024, "evidence exceeds 2 MiB");
    Ok(serde_json::from_slice(&bytes)?)
}

pub(super) fn collect(
    assessment: &tier::Assessment,
    plan: &Value,
    directory: &Path,
) -> Result<Value> {
    ensure!(plan["status"] == "complete", "review plan is incomplete");
    ensure!(
        std::fs::symlink_metadata(directory)?.is_dir(),
        "evidence directory must not be a link"
    );
    let mut reports = Vec::new();
    for role in plan["selected_roles"]
        .as_array()
        .context("missing selected roles")?
    {
        let name = role.as_str().context("invalid role name")?;
        botica_core::review::roles::relative_path(name)?;
        let report = read_json(&directory.join(format!("{name}.json")))?;
        ensure!(report["role"] == *role, "role filename and report disagree");
        reports.push(report);
    }
    let validated = botica_core::review::results::validate(plan, &reports)?;
    verify_references(assessment, &reports)?;
    Ok(validated)
}

fn verify_references(assessment: &tier::Assessment, reports: &[Value]) -> Result<()> {
    for report in reports.iter().filter(|report| report["role"] == "refute") {
        for finding in report["findings"].as_array().into_iter().flatten() {
            for reference in finding["refutation_references"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let revision = reference["revision"]
                    .as_str()
                    .context("missing reference revision")?;
                let path = reference["path"]
                    .as_str()
                    .context("missing reference path")?;
                let line = reference["line"]
                    .as_u64()
                    .context("missing reference line")?;
                let quote = reference["quote"].as_str().context("missing code quote")?;
                let bytes = assessment
                    .git
                    .read_limited(&["show", &format!("{revision}:{path}")], 512 * 1024)?;
                let text = std::str::from_utf8(&bytes).context("reference is not UTF-8")?;
                ensure!(
                    line > 0
                        && !quote.contains(['\r', '\n'])
                        && text
                            .lines()
                            .nth(line as usize - 1)
                            .is_some_and(|text| text.contains(quote)),
                    "refutation quote does not match pinned source"
                );
            }
        }
    }
    Ok(())
}

fn record(assessment: &tier::Assessment, plan: &Value, result: &Value) -> Result<()> {
    let settings: botica_config::HistoryConfig = assessment
        .config
        .table
        .as_ref()
        .and_then(|table| table.get("history"))
        .cloned()
        .map(toml::Value::try_into)
        .transpose()?
        .unwrap_or_default();
    let value = std::env::var(KEY_VARIABLE)
        .ok()
        .filter(|key| !key.is_empty());
    ensure!(
        !settings.require_key || value.is_some(),
        "history requires BOTICA_HISTORY_KEY"
    );
    let key = FingerprintKey::new(value.as_deref())?;
    for role in result["roles"]
        .as_array()
        .context("missing validated roles")?
    {
        let name = role["role"].as_str().context("invalid role")?;
        let started = SystemTime::now();
        let run_id = history::run_id(started);
        let slug = format!("review:{name}");
        let mut findings: Vec<_> = result["all_findings"]
            .as_array()
            .context("missing findings")?
            .iter()
            .filter(|finding| {
                finding["id"]
                    .as_str()
                    .is_some_and(|id| id.starts_with(&format!("{name}-")))
            })
            .map(|finding| FindingLine {
                v: 1,
                run_id: run_id.clone(),
                code: None,
                slug: slug.clone(),
                severity: finding["severity"].as_str().unwrap_or("P1").into(),
                path: finding["path"].as_str().unwrap_or("").into(),
                line: finding["line"].as_u64().unwrap_or(0) as usize,
                fingerprint: key.of_message(
                    &slug,
                    finding["path"].as_str().unwrap_or(""),
                    finding["title"].as_str().unwrap_or(""),
                ),
                occurrence: 0,
                refutation: result["refutations"][finding["id"].as_str().unwrap_or("")]
                    .as_str()
                    .map(str::to_owned),
                reference: finding["id"].as_str().map(str::to_owned),
            })
            .collect();
        history::number_occurrences(&mut findings);
        let mut rules = BTreeMap::new();
        rules.insert(
            slug,
            RuleEntry {
                hits: findings.len(),
                hash: plan["roles"]
                    .as_array()
                    .and_then(|roles| roles.iter().find(|role| role["role"] == name))
                    .and_then(|role| role["definition"]["digest"].as_str())
                    .map(|digest| digest.chars().take(16).collect()),
                ..RuleEntry::default()
            },
        );
        let run = RunLine {
            v: 1,
            run_id,
            started_at: history::unix_seconds(started),
            commit: Some(assessment.git.head.clone()),
            head: Some(assessment.git.head.clone()),
            branch: None,
            botica: env!("CARGO_PKG_VERSION").into(),
            rules_fingerprint: plan["inputs"]["roles_digest"]
                .as_str()
                .context("missing digest")?
                .into(),
            key_id: value.as_ref().map(|_| key.id()),
            mode: "reviewed".into(),
            origin: format!("review:{name}"),
            rules,
            detail: Some(
                json!({"verdict":role["verdict"],"base":plan["inputs"]["base"],
                "roles_digest":plan["inputs"]["roles_digest"],"risk_digest":plan["inputs"]["risk_digest"]}),
            ),
        };
        history::append(&assessment.git.root, &run, &findings)?;
    }
    Ok(())
}

pub(super) fn validate(
    args: &consumers::ConsumerArgs,
    directory: &Path,
    recording: bool,
) -> Result<(Value, Value)> {
    let (assessment, plan) = consumers::collect(args, false)?;
    let result = collect(&assessment, &plan, directory)?;
    if recording {
        record(&assessment, &plan, &result)?;
    }
    Ok((plan, result))
}

pub(super) fn print(result: &Value) -> Result<ExitCode> {
    println!("{}", serde_json::to_string_pretty(result)?);
    Ok(if result["status"] != "complete" {
        ExitCode::from(2)
    } else if result["verdict"] != "PASS" {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

pub(super) fn run(args: &EvidenceArgs) -> Result<ExitCode> {
    let (_, result) = validate(&args.plan, &args.results, args.record)?;
    print(&result)
}
