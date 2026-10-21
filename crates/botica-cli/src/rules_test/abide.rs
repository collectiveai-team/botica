//! A judgment rule before abide: its fixtures in a git repository of their own with a rubric that
//! holds the rule alone, and what `abide audit --json` says of each.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use botica_catalog::Rule;
use botica_core::cache::ensure_directory;
use serde::Deserialize;

use super::expected::{Band, Judged};
use crate::rubric;

const SIDES: [&str; 2] = ["valid", "invalid"];

/// Writes `<judge_dir>/<slug>`, emptied first: the fixtures of `rule` under `valid/` and
/// `invalid/`, `.abide/rubric.json` with the rule alone, and a git repository. `judge_dir` is
/// created with a `.gitignore` of `*` when it is new. Returns the directory and the fixtures,
/// each `valid/<file>` or `invalid/<file>`.
pub fn lay_out(
    judge_dir: &Path,
    rule: &Rule,
    fixtures: &Path,
    readme: &str,
) -> Result<(PathBuf, Vec<String>)> {
    ensure_directory(judge_dir)?;
    let dir = judge_dir.join(&rule.id);
    if dir.exists() {
        fs::remove_dir_all(&dir).with_context(|| format!("cannot empty `{}`", dir.display()))?;
    }
    let mut files = Vec::new();
    for side in SIDES {
        copy(&fixtures.join(side), &dir.join(side), side, &mut files)?;
    }
    files.sort();
    let scope = SIDES.iter().map(|side| format!("{side}/**")).collect();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let rubric = rubric::single(rule, readme, scope, rubric::rfc3339(now));
    let path = dir.join(".abide/rubric.json");
    fs::create_dir_all(dir.join(".abide"))?;
    fs::write(&path, rubric::render(&rubric))
        .with_context(|| format!("cannot write `{}`", path.display()))?;
    let init = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&dir)
        .status()
        .context("cannot run `git init`")?;
    if !init.success() {
        bail!("`git init` failed in `{}`", dir.display());
    }
    Ok((dir, files))
}

fn copy(from: &Path, to: &Path, name: &str, files: &mut Vec<String>) -> Result<()> {
    let entries =
        fs::read_dir(from).with_context(|| format!("cannot read `{}`", from.display()))?;
    fs::create_dir_all(to).with_context(|| format!("cannot create `{}`", to.display()))?;
    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let target = to.join(&file_name);
        if entry.file_type()?.is_dir() {
            copy(
                &entry.path(),
                &target,
                &format!("{name}/{file_name}"),
                files,
            )?;
        } else {
            fs::copy(entry.path(), &target)
                .with_context(|| format!("cannot copy `{}`", entry.path().display()))?;
            files.push(format!("{name}/{file_name}"));
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct Audit {
    #[serde(rename = "byFile")]
    by_file: Vec<FileResult>,
}

#[derive(Deserialize)]
struct FileResult {
    file: String,
    #[serde(default)]
    verdicts: Vec<Verdict>,
}

#[derive(Deserialize)]
struct Verdict {
    #[serde(rename = "ruleId")]
    rule_id: String,
    probability: f64,
    band: Band,
}

/// Runs `command audit --json valid invalid` in `dir` and keeps the verdicts of `rule`, by file.
/// abide exits with 1 when a file breaks a rule, which is no failure of the run.
pub fn audit(command: &[String], dir: &Path, rule: &str) -> Result<BTreeMap<String, Judged>> {
    let (program, args) = command.split_first().context("--abide names no command")?;
    let output = Command::new(program)
        .args(args)
        .args(["audit", "--json"])
        .args(SIDES)
        .current_dir(dir)
        .output()
        .with_context(|| format!("cannot run `{}`", command.join(" ")))?;
    if !matches!(output.status.code(), Some(0 | 1)) {
        bail!(
            "`{} audit` failed: {}",
            command.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let audit: Audit = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("`{} audit --json` printed no report", command.join(" ")))?;
    Ok(audit
        .by_file
        .into_iter()
        .filter_map(|result| {
            let verdict = result.verdicts.into_iter().find(|v| v.rule_id == rule)?;
            Some((
                result.file,
                Judged {
                    band: verdict.band,
                    probability: verdict.probability,
                },
            ))
        })
        .collect())
}
