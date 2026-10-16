//! `botica rules test --judge abide`: abide judges the fixtures of each judgment rule, and botica
//! compares its verdicts with the `expected.json` of the rule.

mod abide;
mod expected;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use botica_catalog::{read_files, Catalog, Rule, When};
use clap::{Args, ValueEnum};

use crate::rule_page::group_of;

/// Where the repository of each test goes, from where botica runs.
const JUDGE_DIR: &str = ".botica/judge";

#[derive(Args)]
pub struct TestArgs {
    /// The judgment rules to test, by slug or code; every judgment rule of the catalog when none
    /// is named.
    rules: Vec<String>,
    /// The judge that answers.
    #[arg(long, value_enum)]
    judge: Judge,
    /// The directory of the catalog.
    #[arg(long, default_value = "lint/rules")]
    catalog: PathBuf,
    /// The command that runs abide, its words split on spaces.
    #[arg(long, default_value = "abide")]
    abide: String,
}

#[derive(Clone, Copy, ValueEnum)]
enum Judge {
    /// `abide audit --json`, in a git repository with the fixtures under `.botica/judge/<slug>`.
    Abide,
}

/// How the fixtures of the rules went: how many matched, and how many were tested.
#[derive(Default)]
struct Tally {
    matched: usize,
    tested: usize,
    failed: bool,
}

pub fn run(args: &TestArgs) -> Result<ExitCode> {
    let Judge::Abide = args.judge;
    let files = read_files(&args.catalog)
        .with_context(|| format!("cannot read the catalog `{}`", args.catalog.display()))?;
    let catalog = Catalog::from_files(&files)?;
    let rules = selected(&catalog, &args.rules)?;
    let command: Vec<String> = args.abide.split_whitespace().map(str::to_owned).collect();
    let mut tally = Tally::default();
    for rule in rules {
        test(args, &command, rule, &mut tally)?;
    }
    println!("{} of {} fixtures as expected", tally.matched, tally.tested);
    Ok(if tally.failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn selected<'c>(catalog: &'c Catalog, named: &[String]) -> Result<Vec<&'c Rule>> {
    if named.is_empty() {
        return Ok(catalog
            .rules()
            .iter()
            .filter(|rule| rule.is_judgment())
            .collect());
    }
    named
        .iter()
        .map(|name| match catalog.resolve(name) {
            None => bail!("`{name}` is not a rule of the catalog"),
            Some(rule) if !rule.is_judgment() => bail!("`{name}` is not a judgment rule"),
            Some(rule) => Ok(rule),
        })
        .collect()
}

fn test(args: &TestArgs, command: &[String], rule: &Rule, tally: &mut Tally) -> Result<()> {
    let slug = &rule.id;
    let when = rule.implementations.first().and_then(|i| i.when);
    if when != Some(When::Edit) {
        println!("skip {slug}: abide audit judges the rules with `when = \"edit\"` only");
        return Ok(());
    }
    let folder = Path::new(group_of(rule)).join(slug);
    let fixtures = args.catalog.join(&folder).join("fixtures");
    let expected_path = fixtures.join("expected.json");
    let text = fs::read_to_string(&expected_path)
        .with_context(|| format!("cannot read `{}`", expected_path.display()))?;
    let expected = match expected::parse(&text) {
        Ok(expected) => expected,
        Err(problem) => return fail(tally, slug, &[problem]),
    };
    let readme = folder.join("README.md").to_string_lossy().into_owned();
    let (dir, files) = abide::lay_out(Path::new(JUDGE_DIR), rule, &fixtures, &readme)?;
    let problems = expected::problems(&expected, &files);
    if !problems.is_empty() {
        return fail(tally, slug, &problems);
    }
    let judged = abide::audit(command, &dir, slug)?;
    for verdict in &expected.verdicts {
        let got = judged.get(&verdict.file).copied();
        let said = got.map_or_else(String::new, |got| {
            format!("{} {:.2}", got.band.word(), got.probability)
        });
        tally.tested += 1;
        match expected::mismatch(verdict, got) {
            None => {
                tally.matched += 1;
                println!("ok   {slug} {}: {said}", verdict.file);
            }
            Some(why) => {
                tally.failed = true;
                let said = if said.is_empty() {
                    why
                } else {
                    format!("{said}, {why}")
                };
                println!("FAIL {slug} {}: {said}", verdict.file);
            }
        }
    }
    Ok(())
}

fn fail(tally: &mut Tally, slug: &str, problems: &[String]) -> Result<()> {
    tally.failed = true;
    for problem in problems {
        println!("FAIL {slug}: {problem}");
    }
    Ok(())
}
