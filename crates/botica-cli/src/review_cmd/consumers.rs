use std::process::ExitCode;

use anyhow::{Context, Result};
use botica_core::review::roles::RoleDefinition;
use clap::{Args, ValueEnum};
use serde_json::{json, Value};

use super::tier;
#[path = "consumer_checks.rs"]
mod checks;
#[path = "consumer_coverage.rs"]
mod coverage;
#[path = "consumer_sources.rs"]
mod sources;

#[derive(Args)]
pub(super) struct ConsumerArgs {
    #[command(flatten)]
    inputs: tier::TierArgs,
    #[arg(long, value_enum, default_value = "local")]
    stage: Stage,
}

#[derive(Clone, Copy, ValueEnum)]
enum Stage {
    Local,
    Ci,
}

impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ci => "ci",
        }
    }
}

fn role(
    assessment: &tier::Assessment,
    directory: &str,
    name: &str,
    stage: &str,
    checking: bool,
) -> Result<(Value, Vec<Value>)> {
    let path = format!("{directory}/{name}/REVIEW.md");
    let bytes = assessment
        .git
        .blob(&path)?
        .with_context(|| format!("missing trusted role {path}"))?;
    let definition = RoleDefinition::parse(name, std::str::from_utf8(&bytes)?)?;
    let selection = definition.selection(&assessment.report, stage)?;
    let mut reasons = Vec::new();
    if checking {
        reasons.extend(checks::unmatched(
            &definition,
            &assessment.git.merge_paths()?,
        ));
    }
    let covered = match coverage::inspect(assessment, &definition, std::str::from_utf8(&bytes)?) {
        Ok(covered) => {
            if covered["status"] != "current" {
                reasons.push(json!({"code": "stale-coverage", "role": name, "message": "Trusted coverage block differs from its declarations"}));
            }
            covered
        }
        Err(error) => {
            reasons.push(json!({"code": "unverified-coverage", "role": name, "message": format!("{error:#}")}));
            json!({"status": "unverified", "expected_block": null, "references": []})
        }
    };
    let skill = if checking {
        match checks::skill(&assessment.git.root, name) {
            Ok(skill) => Some(skill),
            Err(error) => {
                reasons.push(
                    json!({"code": "missing-skill", "role": name, "message": format!("{error:#}")}),
                );
                None
            }
        }
    } else {
        None
    };
    Ok((
        json!({"role": name, "definition": {"path": path, "digest": blake3::hash(&bytes).to_hex().to_string()},
        "selection": selection, "skill": skill, "covered": covered}),
        reasons,
    ))
}

struct Policy {
    roles: Vec<Value>,
    inputs: Value,
    reasons: Vec<Value>,
}

fn print_text(report: &Value) {
    println!(
        "review {}: {}",
        report["command"].as_str().unwrap_or(""),
        report["status"].as_str().unwrap_or("")
    );
    println!(
        "risk classification: {} ({})",
        report["risk"]["status"].as_str().unwrap_or(""),
        report["risk"]["effective_tier"]
            .as_str()
            .unwrap_or("unavailable")
    );
    println!("inputs: {}", report["inputs"]);
    for role in report["roles"].as_array().into_iter().flatten() {
        let selection = match role["selection"]["selected"].as_bool() {
            Some(true) if report["command"] == "plan" && report["status"] == "complete" => {
                "selected"
            }
            Some(true) => "criteria match (no final selection)",
            Some(false) => "skipped",
            None => "unavailable",
        };
        println!("{}: {selection}", role["role"].as_str().unwrap_or(""));
        for field in ["definition", "selection", "covered", "skill"] {
            let label = if field == "covered" {
                "coverage"
            } else {
                field
            };
            println!("  {label}: {}", role[field]);
        }
    }
    for reason in report["reasons"].as_array().into_iter().flatten() {
        println!("reason: {reason}");
    }
    println!("This command does not authorize merge.");
}

fn policy(assessment: &tier::Assessment, args: &ConsumerArgs, checking: bool) -> Result<Policy> {
    let mut reasons = Vec::new();
    let mut roles = Vec::new();
    let mut inputs = assessment.report["inputs"].clone();
    let config = assessment
        .review
        .as_ref()
        .context("trusted review settings are unavailable")?;
    let directory = sources::directory(assessment, config)?;
    match sources::bindings(assessment, &directory, config) {
        Ok(bound) => inputs = bound,
        Err(error) => {
            reasons.push(json!({"code": "invalid-policy-sources", "message": format!("{error:#}")}))
        }
    }
    inputs["stage"] = json!(args.stage.name());
    inputs["review_directory"] = json!(directory);
    for expected in config.expected_roles() {
        match role(assessment, &directory, expected.name(), args.stage.name(), checking) {
            Ok((role, found)) => { roles.push(role); reasons.extend(found); },
            Err(error) => reasons.push(json!({"code": "invalid-role", "role": expected.name(), "message": format!("{error:#}")})),
        }
    }
    Ok(Policy {
        roles,
        inputs,
        reasons,
    })
}

pub(super) fn run(args: &ConsumerArgs, checking: bool) -> Result<ExitCode> {
    let assessment = tier::collect(&args.inputs)?;
    let Policy {
        roles,
        inputs,
        mut reasons,
    } = policy(&assessment, args, checking).unwrap_or_else(|error| Policy {
        roles: vec![],
        inputs: assessment.report["inputs"].clone(),
        reasons: vec![json!({"code": "invalid-policy-sources", "message": format!("{error:#}")})],
    });
    if assessment.report["status"] != "complete" && !checking {
        reasons.push(json!({"code": "incomplete-risk", "message": "A complete classification is required to select roles"}));
    }
    let done = reasons.is_empty();
    let selected = if done && !checking {
        Some(
            roles
                .iter()
                .filter(|role| role["selection"]["selected"] == true)
                .map(|role| role["role"].clone())
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let report = json!({"schema_version": 1, "command": if checking { "check" } else { "plan" },
        "status": if done { "complete" } else { "incomplete" }, "inputs": inputs,
        "stage": args.stage.name(), "selected_roles": selected, "roles": roles, "reasons": reasons,
        "risk": assessment.report});
    match args.inputs.format {
        tier::Format::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        tier::Format::Text => print_text(&report),
    }
    Ok(if done {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}
