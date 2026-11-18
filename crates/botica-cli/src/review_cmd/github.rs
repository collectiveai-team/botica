//! Read GitHub-owned context and check conclusions through the authenticated gh client.

use anyhow::{ensure, Context, Result};
use clap::Args;
use serde_json::{json, Value};
use std::io::Read;
use std::process::{Command, Stdio};

#[path = "github_reviews.rs"]
mod observations;
pub(super) use observations::reviews;

#[derive(Args)]
pub(super) struct ContextArgs {
    #[arg(long)]
    pub(super) repository: String,
    #[arg(long)]
    pub(super) pr: u64,
    #[arg(long)]
    pub(super) base: String,
    #[arg(long)]
    pub(super) head: String,
}

fn valid_repository(repository: &str) -> bool {
    let parts: Vec<_> = repository.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|part| {
            part.bytes().any(|byte| byte.is_ascii_alphanumeric())
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
}

pub(super) fn automation_policy(repository: &str, pr: u64, inputs: &Value) -> Result<bool> {
    let pull = pull(repository, pr)?;
    matches(
        &pull,
        inputs["base"].as_str().context("missing base")?,
        inputs["head"].as_str().context("missing head")?,
    )?;
    let branch = pull["base"]["ref"]
        .as_str()
        .context("missing destination branch")?;
    let branch: String = branch
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-_.".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    let protection = gh(&[
        "api",
        &format!("repos/{repository}/branches/{branch}/protection/required_status_checks"),
    ])?;
    let app = gh(&["api", "apps/github-actions"])?;
    Ok(protection["strict"] == true
        && app["id"].is_u64()
        && protection["checks"].as_array().is_some_and(|checks| {
            checks.iter().any(|check| {
                check["context"] == "botica-authorization" && check["app_id"] == app["id"]
            })
        }))
}

fn gh(args: &[&str]) -> Result<Value> {
    serde_json::from_slice(&gh_bytes(args)?)
        .map_err(|_| anyhow::anyhow!("GitHub returned invalid JSON"))
}

fn gh_bytes(args: &[&str]) -> Result<Vec<u8>> {
    let mut child = Command::new("gh")
        .args(args)
        .env_remove("OPENAI_API_KEY")
        .env_remove("TYPESAFE_AI_API_KEY")
        .env_remove("BOTICA_HISTORY_KEY")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("GitHub client is unavailable")?;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .context("missing GitHub output")?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > 8 * 1024 * 1024 {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("GitHub evidence exceeded its bound or could not be read");
    }
    ensure!(child.wait()?.success(), "GitHub evidence request failed");
    Ok(bytes)
}

pub(super) fn verify_roles(
    repository: &str,
    pr: u64,
    assessment: &super::tier::Assessment,
    plan: &Value,
    results: &Value,
) -> Result<Value> {
    ensure!(
        valid_repository(repository) && pr > 0,
        "invalid GitHub repository or PR"
    );
    let response = gh(&["api", &format!("repos/{repository}/actions/workflows/review.yml/runs?event=pull_request_target&per_page=100")])?;
    let run = response["workflow_runs"]
        .as_array()
        .context("missing review runs")?
        .iter()
        .filter(|run| {
            run["event"] == "pull_request_target"
                && run["head_repository"]["full_name"] == repository
                && run["pull_requests"].as_array().is_some_and(|pulls| {
                    pulls.iter().any(|pull| {
                        pull["number"] == pr
                            && pull["head"]["sha"] == plan["inputs"]["head"]
                            && pull["base"]["sha"] == plan["inputs"]["base"]
                    })
                })
        })
        .max_by_key(|run| run["id"].as_u64().unwrap_or(0))
        .context("no associated role run")?;
    ensure!(
        run["status"] == "completed" && run["conclusion"] == "success",
        "latest associated role run did not succeed"
    );
    let run_id = run["id"].as_u64().context("missing role run ID")?;
    let artifacts = gh(&[
        "api",
        &format!("repos/{repository}/actions/runs/{run_id}/artifacts?per_page=100"),
    ])?;
    ensure!(
        artifacts["total_count"]
            .as_u64()
            .is_some_and(|count| count <= 100),
        "role artifacts exceed their bound"
    );
    let found: Vec<_> = artifacts["artifacts"]
        .as_array()
        .context("missing role artifacts")?
        .iter()
        .filter(|artifact| artifact["name"] == "review-evidence")
        .collect();
    ensure!(
        found.len() == 1
            && found[0]["expired"] == false
            && found[0]["size_in_bytes"]
                .as_u64()
                .is_some_and(|size| size <= 16 * 1024 * 1024),
        "role artifact is unavailable or oversized"
    );
    let temporary = assessment.git.root.join(".tmp");
    std::fs::create_dir_all(&temporary)?;
    ensure!(
        std::fs::symlink_metadata(&temporary)?.is_dir(),
        "temporary directory must not be a link"
    );
    let directory = temporary.join(format!(
        "github-review-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    std::fs::create_dir(&directory)?;
    let verified = (|| -> Result<Value> {
        gh_bytes(&[
            "run",
            "download",
            &run_id.to_string(),
            "--repo",
            repository,
            "--name",
            "review-evidence",
            "--dir",
            directory.to_str().context("invalid artifact path")?,
        ])?;
        let trusted = super::evidence::collect(assessment, plan, &directory.join(".tmp/results"))?;
        let mut expected = results.clone();
        if let Some(object) = expected.as_object_mut() {
            object.remove("producer_verified");
            object.remove("provenance");
        }
        ensure!(
            trusted == expected,
            "local role evidence differs from the GitHub-owned artifact"
        );
        Ok(
            json!({"source":"github","repository":repository,"pr":pr,"run_id":run_id,
            "artifact_id":found[0]["id"],"producer_verified":true}),
        )
    })();
    std::fs::remove_dir_all(&directory)?;
    verified
}

fn pull(repository: &str, pr: u64) -> Result<Value> {
    ensure!(
        valid_repository(repository) && pr > 0,
        "invalid GitHub repository or PR"
    );
    gh(&["api", &format!("repos/{repository}/pulls/{pr}")])
}

fn matches(pull: &Value, base: &str, head: &str) -> Result<()> {
    ensure!(
        pull["base"]["sha"] == base && pull["head"]["sha"] == head,
        "GitHub PR inputs changed"
    );
    Ok(())
}

pub(super) fn context(args: &ContextArgs) -> Result<Value> {
    let original = pull(&args.repository, args.pr)?;
    matches(&original, &args.base, &args.head)?;
    let (owner, name) = args
        .repository
        .split_once('/')
        .context("invalid repository")?;
    let query = "query($owner:String!,$name:String!,$number:Int!){repository(owner:$owner,name:$name){pullRequest(number:$number){closingIssuesReferences(first:100){nodes{number title body url} pageInfo{hasNextPage}}}}}";
    let response = gh(&[
        "api",
        "graphql",
        "-f",
        &format!("query={query}"),
        "-f",
        &format!("owner={owner}"),
        "-f",
        &format!("name={name}"),
        "-F",
        &format!("number={}", args.pr),
    ])?;
    let linked = &response["data"]["repository"]["pullRequest"]["closingIssuesReferences"];
    ensure!(
        linked["pageInfo"]["hasNextPage"] == false && linked["nodes"].is_array(),
        "linked issue context is incomplete"
    );
    matches(&pull(&args.repository, args.pr)?, &args.base, &args.head)?;
    let description = original["body"].as_str().unwrap_or("");
    let acceptance = description
        .split_once("## Acceptance criteria")
        .map(|(_, section)| section.split("\n## ").next().unwrap_or("").trim())
        .filter(|section| !section.is_empty());
    Ok(
        json!({"title":original["title"],"description":original["body"],"issues":linked["nodes"],
        "acceptance_criteria":acceptance.into_iter().collect::<Vec<_>>(),
        "repository":args.repository,"pr":args.pr,"base":args.base,"head":args.head,"source":"github"}),
    )
}

pub(super) fn context_matches(repository: &str, pr: u64, plan: &Value) -> Result<bool> {
    let current = context(&ContextArgs {
        repository: repository.into(),
        pr,
        base: plan["inputs"]["base"]
            .as_str()
            .context("missing base")?
            .into(),
        head: plan["inputs"]["head"]
            .as_str()
            .context("missing head")?
            .into(),
    })?;
    Ok(current == plan["untrusted_context"])
}

pub(super) fn checks(repository: &str, pr: u64, plan: &Value) -> Result<Value> {
    let base = plan["inputs"]["base"].as_str().context("missing base")?;
    let head = plan["inputs"]["head"].as_str().context("missing head")?;
    let original = pull(repository, pr)?;
    ensure!(
        original["state"] == "open" && original["draft"] == false,
        "PR is not open and ready"
    );
    matches(&original, base, head)?;
    let check_head = original["merge_commit_sha"]
        .as_str()
        .filter(|value| !value.is_empty())
        .context("GitHub has no current integration check head")?;
    let commit = gh(&[
        "api",
        &format!("repos/{repository}/git/commits/{check_head}"),
    ])?;
    ensure!(
        commit["tree"]["sha"] == plan["risk"]["integration"]["comparisons"][0]["to"],
        "GitHub checked tree differs from the assessed integration result"
    );
    let mut runs = Vec::new();
    let mut page = 1;
    loop {
        ensure!(page <= 100, "GitHub check pagination exceeded its bound");
        let response = gh(&["api",&format!("repos/{repository}/commits/{check_head}/check-runs?per_page=100&page={page}&filter=latest")])?;
        let found = response["check_runs"]
            .as_array()
            .context("missing check runs")?;
        runs.extend(found.iter().cloned());
        if found.len() < 100 {
            break;
        }
        page += 1;
    }
    let required = plan["required_checks"]
        .as_array()
        .context("missing required checks")?;
    let mut jobs = serde_json::Map::new();
    let mut provenance: Vec<_> = runs.iter().map(|run| json!({"id":run["id"],"name":run["name"],
        "head":run["head_sha"],"url":run["html_url"],"status":run["status"],"conclusion":run["conclusion"]})).collect();
    for name in required.iter().filter_map(Value::as_str).chain(["mutants"]) {
        let found: Vec<_> = runs.iter().filter(|run| run["name"] == name).collect();
        let passed = found.len() == 1
            && found[0]["head_sha"] == check_head
            && found[0]["status"] == "completed"
            && found[0]["conclusion"] == "success"
            && found[0]["app"]["slug"] == "github-actions";
        jobs.insert(
            name.into(),
            json!(if passed { "PASS" } else { "INCONCLUSIVE" }),
        );
    }
    let statuses = commit_statuses(repository, check_head)?;
    provenance.extend(statuses.iter().map(|status| {
        json!({"name":status["context"],
        "head":check_head,"url":status["target_url"],"status":status["state"]})
    }));
    let current = pull(repository, pr)?;
    matches(&current, base, head)?;
    ensure!(
        current["merge_commit_sha"] == check_head,
        "GitHub integration check head changed"
    );
    let ci_pass = !required.is_empty()
        && runs
            .iter()
            .filter(|run| {
                !(run["name"] == "botica-authorization" && run["app"]["slug"] == "github-actions")
            })
            .all(|run| {
                run["head_sha"] == check_head
                    && run["status"] == "completed"
                    && matches!(
                        run["conclusion"].as_str(),
                        Some("success" | "neutral" | "skipped")
                    )
            })
        && statuses.iter().all(|status| status["state"] == "success")
        && required
            .iter()
            .filter_map(Value::as_str)
            .all(|name| jobs[name] == "PASS");
    Ok(
        json!({"inputs":plan["inputs"],"producer_verified":true,"source":"github","repository":repository,
        "pr":pr,"check_head":check_head,"ci":if ci_pass {"PASS"} else {"INCONCLUSIVE"},
        "lint":if ci_pass {"PASS"} else {"INCONCLUSIVE"},"mutation":jobs["mutants"],"jobs":jobs,"provenance":provenance}),
    )
}

fn commit_statuses(repository: &str, head: &str) -> Result<Vec<Value>> {
    let mut statuses = Vec::new();
    for page in 1..=100 {
        let response = gh(&[
            "api",
            &format!("repos/{repository}/commits/{head}/status?per_page=100&page={page}"),
        ])?;
        let found = response["statuses"]
            .as_array()
            .context("missing commit statuses")?;
        statuses.extend(found.iter().cloned());
        if found.len() < 100 {
            return Ok(statuses);
        }
    }
    anyhow::bail!("GitHub status pagination exceeded its bound")
}
