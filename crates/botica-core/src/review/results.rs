//! Validate role evidence against a fresh trusted plan, never the model's verdict alone.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::roles::relative_path;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub severity: String,
    pub path: String,
    pub line: Option<u64>,
    pub title: String,
    pub evidence: String,
    pub reproduced: String,
    pub how_to_verify: Vec<String>,
    pub fix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refutation: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Command {
    run: String,
    result: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RoleResult {
    role: String,
    head: String,
    base: String,
    inputs: Value,
    verdict: String,
    findings: Vec<Finding>,
    checked: Vec<String>,
    not_checked: Vec<String>,
    commands: Vec<Command>,
}

fn severity(value: &str) -> Result<u8> {
    ["P0", "P1", "P2", "P3"]
        .iter()
        .position(|candidate| *candidate == value)
        .map(|index| index as u8)
        .context("invalid finding severity")
}

fn finding(finding: &Finding) -> Result<()> {
    relative_path(&finding.path)?;
    severity(&finding.severity)?;
    ensure!(
        finding.line.is_none_or(|line| line > 0)
            && !finding.id.trim().is_empty()
            && !finding.title.trim().is_empty()
            && !finding.evidence.trim().is_empty(),
        "finding lacks evidence or location"
    );
    ensure!(
        matches!(finding.reproduced.as_str(), "yes" | "no" | "not-tried"),
        "invalid reproduced value"
    );
    Ok(())
}

fn read(plan: &Value, reports: &[Value]) -> Result<BTreeMap<String, RoleResult>> {
    ensure!(plan["status"] == "complete", "review plan is incomplete");
    let expected: BTreeSet<_> = plan["selected_roles"]
        .as_array()
        .context("missing selected roles")?
        .iter()
        .map(|role| role.as_str().context("invalid selected role"))
        .collect::<Result<_>>()?;
    let mut found = BTreeMap::new();
    for report in reports {
        let result: RoleResult = serde_json::from_value(report.clone())?;
        ensure!(
            expected.contains(result.role.as_str()),
            "unexpected review role"
        );
        ensure!(
            result.inputs == plan["inputs"],
            "stale review policy or risk evidence"
        );
        ensure!(
            result.head == plan["inputs"]["head"] && result.base == plan["inputs"]["base"],
            "stale review base or head"
        );
        ensure!(
            matches!(result.verdict.as_str(), "PASS" | "FAIL" | "INCONCLUSIVE"),
            "invalid verdict"
        );
        ensure!(
            !result.checked.is_empty() || result.verdict == "INCONCLUSIVE",
            "empty review coverage"
        );
        let policy = plan["roles"]
            .as_array()
            .context("missing role policies")?
            .iter()
            .find(|role| role["role"] == result.role)
            .context("missing role policy")?;
        ensure!(
            result.role == "refute"
                || result.findings.len() as u64
                    <= policy["max_findings"]
                        .as_u64()
                        .context("missing finding limit")?,
            "too many role findings"
        );
        let mut ids = BTreeSet::new();
        for item in &result.findings {
            finding(item)?;
            ensure!(ids.insert(&item.id), "duplicate finding id");
            if result.role != "refute" {
                ensure!(
                    item.id.starts_with(&format!("{}-", result.role)),
                    "finding id has wrong role"
                );
                ensure!(item.refutation.is_none(), "only refute may refute findings");
            }
        }
        ensure!(
            found.insert(result.role.clone(), result).is_none(),
            "duplicate review role"
        );
    }
    ensure!(
        found.keys().map(String::as_str).collect::<BTreeSet<_>>() == expected,
        "missing review role"
    );
    Ok(found)
}

fn refutations(results: &BTreeMap<String, RoleResult>) -> Result<BTreeMap<String, String>> {
    let originals: BTreeMap<_, _> = results
        .values()
        .filter(|role| role.role != "refute")
        .flat_map(|role| {
            role.findings
                .iter()
                .map(|finding| (finding.id.clone(), finding))
        })
        .collect();
    let mut dispositions = BTreeMap::new();
    if let Some(refute) = results.get("refute") {
        ensure!(
            refute.findings.len() == originals.len(),
            "refute must preserve every input finding"
        );
        for item in &refute.findings {
            let original = originals
                .get(&item.id)
                .context("refute invented a finding")?;
            let mut comparable = item.clone();
            comparable.refutation = None;
            ensure!(&comparable == *original, "refute rewrote original evidence");
            if let Some(disposition) = &item.refutation {
                ensure!(
                    matches!(
                        disposition.as_str(),
                        "confirmed"
                            | "pre-existing"
                            | "unreachable"
                            | "not-reproduced"
                            | "duplicate"
                    ),
                    "invalid refutation"
                );
                dispositions.insert(item.id.clone(), disposition.clone());
            }
        }
    }
    Ok(dispositions)
}

/// Return recomputed role verdicts and surviving findings, rejecting missing or stale evidence.
pub fn validate(plan: &Value, reports: &[Value]) -> Result<Value> {
    let results = read(plan, reports)?;
    let dispositions = refutations(&results)?;
    let mut roles = Vec::new();
    let mut surviving = Vec::new();
    let mut all_findings = Vec::new();
    let mut failed = false;
    let mut incomplete = false;
    for result in results.values() {
        let policy = plan["roles"]
            .as_array()
            .context("missing policies")?
            .iter()
            .find(|role| role["role"] == result.role)
            .context("missing role")?;
        let threshold = severity(
            policy["blocking"]
                .as_str()
                .context("missing blocking severity")?,
        )?;
        let mut blocking = false;
        if result.role != "refute" {
            for item in &result.findings {
                all_findings.push(item);
                if dispositions
                    .get(&item.id)
                    .is_none_or(|value| value == "confirmed")
                {
                    let rank = severity(&item.severity)?;
                    blocking |= rank < 3 && rank <= threshold;
                    surviving.push(item);
                }
            }
        }
        let inconclusive = result.verdict == "INCONCLUSIVE";
        incomplete |= inconclusive;
        failed |= blocking;
        roles.push(json!({"role":result.role, "verdict":if inconclusive {"INCONCLUSIVE"} else if blocking {"FAIL"} else {"PASS"},
            "checked":result.checked, "not_checked":result.not_checked, "commands":result.commands}));
    }
    Ok(
        json!({"schema_version":1,"status":if incomplete {"incomplete"} else {"complete"},
        "inputs":plan["inputs"],"verdict":if incomplete {"INCONCLUSIVE"} else if failed {"FAIL"} else {"PASS"},
        "roles":roles,"findings":surviving,"all_findings":all_findings,"refutations":dispositions}),
    )
}
