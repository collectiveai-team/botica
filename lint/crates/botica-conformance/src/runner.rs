use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use crate::fixture_diagnostic::FixtureDiagnostic;
use crate::outcome::{Failure, Outcome};
use crate::side::Side;

/// One way of running botica whose findings must not differ from the others': the optimisations
/// that can be switched off must not change what a rule finds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Variant {
    name: String,
    args: Vec<String>,
}

impl Variant {
    fn new(name: &str, args: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            args: args.iter().map(|&arg| arg.to_owned()).collect(),
        }
    }

    /// The default run and one run with each optimisation off.
    fn standard() -> Vec<Self> {
        vec![
            Self::new("default", &[]),
            Self::new("no-prefilter", &["--no-prefilter"]),
            Self::new("no-regions", &["--no-regions"]),
            Self::new("no-subtree-skip", &["--no-subtree-skip"]),
        ]
    }
}

pub struct Runner {
    binary: PathBuf,
    scratch: PathBuf,
    variants: Vec<Variant>,
    /// The catalog of a project that the rules checked belong to: its name and its folder. Each
    /// project the runner builds holds a copy of it in `.botica/rules/<name>`.
    project_catalog: Option<(String, PathBuf)>,
}

impl Runner {
    pub fn new(binary: impl Into<PathBuf>, scratch: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            scratch: scratch.into(),
            variants: Variant::standard(),
            project_catalog: None,
        }
    }

    /// Checks every rule of a catalog directory (`<family>/<slug>/rule.toml`, the slug being the
    /// rule's ID) that `botica lint` runs, in slug order. A judge or another tool evaluates the
    /// rules whose implementations are all of kind `judgment` or `external`.
    pub fn check_catalog(&self, rules_dir: &Path) -> Result<Vec<Outcome>> {
        rule_folders(rules_dir)?
            .iter()
            .map(|rule| {
                let slug = rule.file_name().and_then(|name| name.to_str());
                self.check(
                    slug.context("a rule folder name is not UTF-8")?,
                    &rule.join("fixtures"),
                )
            })
            .collect()
    }

    /// Like `check_catalog` for a catalog of a project, `.botica/rules/<catalog>`. The rule ids
    /// are `<catalog>/<slug>`, and each project the runner builds holds a copy of the catalog.
    pub fn check_project_catalog(&self, catalog_dir: &Path) -> Result<Vec<Outcome>> {
        let name = catalog_dir
            .file_name()
            .and_then(|name| name.to_str())
            .context("the folder of a catalog has no UTF-8 name")?;
        let runner = Self {
            binary: self.binary.clone(),
            scratch: self.scratch.clone(),
            variants: self.variants.clone(),
            project_catalog: Some((name.to_owned(), catalog_dir.to_owned())),
        };
        rule_folders(catalog_dir)?
            .iter()
            .map(|rule| {
                let slug = rule.file_name().and_then(|name| name.to_str());
                runner.check(
                    &format!(
                        "{name}/{}",
                        slug.context("a rule folder name is not UTF-8")?
                    ),
                    &rule.join("fixtures"),
                )
            })
            .collect()
    }

    /// Runs `rule` over `fixtures/valid` and `fixtures/invalid` and compares with
    /// `fixtures/expected.json`.
    pub fn check(&self, rule: &str, fixtures: &Path) -> Result<Outcome> {
        let mut failures = Vec::new();
        self.check_valid(rule, fixtures, &mut failures)?;
        self.check_invalid(rule, fixtures, &mut failures)?;
        Ok(Outcome::new(rule, failures))
    }

    fn check_valid(&self, rule: &str, fixtures: &Path, failures: &mut Vec<Failure>) -> Result<()> {
        let valid = fixtures.join(Side::Valid.folder());
        if !holds_files(&valid) {
            failures.push(Failure::NoFiles { side: Side::Valid });
            return Ok(());
        }
        for variant in &self.variants {
            for finding in self.run(rule, variant, &valid, Side::Valid)? {
                failures.push(Failure::Unexpected {
                    variant: variant.name.clone(),
                    finding,
                });
            }
        }
        Ok(())
    }

    fn check_invalid(
        &self,
        rule: &str,
        fixtures: &Path,
        failures: &mut Vec<Failure>,
    ) -> Result<()> {
        let invalid = fixtures.join(Side::Invalid.folder());
        if !holds_files(&invalid) {
            failures.push(Failure::NoFiles {
                side: Side::Invalid,
            });
            return Ok(());
        }
        let expected_path = fixtures.join("expected.json");
        if !expected_path.is_file() {
            failures.push(Failure::NoExpected);
            return Ok(());
        }
        let expected = read_expected(&expected_path)?;
        if expected.is_empty() {
            failures.push(Failure::ExpectsNothing);
            return Ok(());
        }
        for variant in &self.variants {
            let actual = self.run(rule, variant, &invalid, Side::Invalid)?;
            if actual != expected {
                failures.push(Failure::Mismatch {
                    variant: variant.name.clone(),
                    expected: expected.clone(),
                    actual,
                });
            }
        }
        Ok(())
    }

    /// The findings of `rule` when botica lints a copy of `source` as a project of its own.
    fn run(
        &self,
        rule: &str,
        variant: &Variant,
        source: &Path,
        side: Side,
    ) -> Result<Vec<FixtureDiagnostic>> {
        let project = self
            .scratch
            .join(rule)
            .join(&variant.name)
            .join(side.folder());
        let _ = fs::remove_dir_all(&project);
        copy_tree(source, &project)?;
        if let Some((name, catalog)) = &self.project_catalog {
            copy_tree(catalog, &project.join(".botica/rules").join(name))?;
        }
        fs::create_dir_all(project.join(".git"))?;
        let output = Command::new(&self.binary)
            .args([
                "lint",
                "--preview",
                "--output-format",
                "json",
                "--extend-select",
            ])
            .arg(rule)
            .args(&variant.args)
            .args(given_paths(&project)?)
            .current_dir(&project)
            .output()
            .with_context(|| format!("run {}", self.binary.display()))?;
        if !matches!(output.status.code(), Some(0 | 1)) {
            bail!(
                "botica failed on {}: {}",
                source.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let report: Value = serde_json::from_slice(&output.stdout)
            .with_context(|| format!("the output for {} is not JSON", source.display()))?;
        diagnostics_of(&report, rule)
    }
}

/// The rule folders of a catalog directory that `botica lint` runs, in slug order.
fn rule_folders(rules_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut rules = Vec::new();
    for family in subdirectories(rules_dir)? {
        for rule in subdirectories(&family)? {
            let toml = rule.join("rule.toml");
            if toml.is_file() && runs_in_lint(&toml)? {
                rules.push(rule);
            }
        }
    }
    rules.sort_by_key(|rule| rule.file_name().map(ToOwned::to_owned));
    Ok(rules)
}

/// What botica is told to lint: the paths one per line in `cli-args.txt`, as a command line would
/// give them (some rules only exist for those), or else the whole project.
fn given_paths(project: &Path) -> Result<Vec<String>> {
    let listing = project.join("cli-args.txt");
    if !listing.is_file() {
        return Ok(vec![".".to_owned()]);
    }
    let text =
        fs::read_to_string(&listing).with_context(|| format!("read {}", listing.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

fn diagnostics_of(report: &Value, rule: &str) -> Result<Vec<FixtureDiagnostic>> {
    report["diagnostics"]
        .as_array()
        .context("the report has no `diagnostics` list")?
        .iter()
        .filter(|diagnostic| diagnostic["rule"] == rule)
        .map(FixtureDiagnostic::from_json)
        .collect()
}

fn read_expected(path: &Path) -> Result<Vec<FixtureDiagnostic>> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let document: Value =
        serde_json::from_str(&text).with_context(|| format!("{} is not JSON", path.display()))?;
    document["diagnostics"]
        .as_array()
        .with_context(|| format!("{} has no `diagnostics` list", path.display()))?
        .iter()
        .map(FixtureDiagnostic::from_json)
        .collect()
}

/// Whether one implementation of the rule is of a kind that `botica lint` runs.
fn runs_in_lint(rule_toml: &Path) -> Result<bool> {
    let text =
        fs::read_to_string(rule_toml).with_context(|| format!("read {}", rule_toml.display()))?;
    let rule: toml::Table = text
        .parse()
        .with_context(|| format!("{} is not TOML", rule_toml.display()))?;
    let implementations = rule
        .get("implementations")
        .and_then(toml::Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    Ok(implementations.iter().any(|implementation| {
        let kind = implementation.get("kind").and_then(toml::Value::as_str);
        !matches!(kind, Some("judgment" | "external"))
    }))
}

fn subdirectories(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            found.push(path);
        }
    }
    Ok(found)
}

fn holds_files(dir: &Path) -> bool {
    fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            let path = entry.path();
            path.is_file() || holds_files(&path)
        })
    })
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("read {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}
