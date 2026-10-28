//! The workflows of the built-in recipes, as `install` writes them with the defaults of their
//! parameters, pass actionlint and zizmor.
//!
//! zizmor and actionlint collect only `.github/workflows/`, so the templates under
//! `catalog/recipes/files/` are not checked by the workflows of this repository. Each recipe is
//! written to a directory of its own, with the `.github/zizmor.yml` of the recipe `ci`, and the two
//! tools run there as the workflow `zizmor.yml` of that recipe runs them. The directories are inside
//! this repository, so the files and the configuration are passed explicitly: both tools would
//! otherwise go up to the root of the repository and read its workflows or its configuration. A
//! test skips a tool that is not installed, unless `BOTICA_REQUIRE_WORKFLOW_LINTERS` is set (CI).

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use botica_catalog::Recipes;
use common::workflows::text;

const WORKFLOWS: &str = ".github/workflows/";

/// Writes the workflows of each built-in recipe under a directory of its own, and returns the
/// directories with the number of workflows written.
fn write_workflows(name: &str) -> Vec<(String, PathBuf)> {
    let recipes = Recipes::embedded().expect("the built-in recipes load");
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&root);
    let zizmor_config = text("ci", "files/github/zizmor.yml");
    let mut written = Vec::new();
    for recipe in recipes.recipes() {
        let files: Vec<_> = recipe
            .files
            .iter()
            .filter(|file| file.path.starts_with(WORKFLOWS))
            .collect();
        if files.is_empty() {
            continue;
        }
        let dir = root.join(&recipe.id);
        fs::create_dir_all(dir.join(WORKFLOWS)).expect("the directory of the workflows");
        fs::write(dir.join(".github/zizmor.yml"), &zizmor_config).expect("the zizmor config");
        for file in files {
            fs::write(dir.join(&file.path), text(&recipe.id, &file.template))
                .expect("the workflow");
        }
        written.push((recipe.id.clone(), dir));
    }
    assert!(written.len() > 5, "the recipes of CI add workflows");
    written
}

/// Runs `program` in `dir`, or returns None when it is not installed and the linters are optional.
fn run(program: &str, args: &[&str], dir: &Path) -> Option<std::process::Output> {
    match Command::new(program).args(args).current_dir(dir).output() {
        Ok(output) => Some(output),
        Err(error) if std::env::var_os("BOTICA_REQUIRE_WORKFLOW_LINTERS").is_some() => {
            panic!("BOTICA_REQUIRE_WORKFLOW_LINTERS is set but `{program}` failed to run: {error}")
        }
        Err(_) => {
            eprintln!("skipped: `{program}` is not installed");
            None
        }
    }
}

fn failures(program: &str, args: &[&str], name: &str) -> Option<Vec<String>> {
    let mut failed = Vec::new();
    for (recipe, dir) in write_workflows(name) {
        let mut workflows: Vec<String> = fs::read_dir(dir.join(WORKFLOWS))
            .expect("the workflows")
            .map(|entry| {
                format!(
                    "{WORKFLOWS}{}",
                    entry.expect("an entry").file_name().to_string_lossy()
                )
            })
            .collect();
        workflows.sort();
        let mut all: Vec<&str> = args.to_vec();
        all.extend(workflows.iter().map(String::as_str));
        let output = run(program, &all, &dir)?;
        if !output.status.success() {
            failed.push(format!(
                "{recipe}:\n{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    Some(failed)
}

#[test]
fn the_workflows_of_the_recipes_pass_actionlint() {
    // actionlint 1.7.12 does not know the self-repository syntax `uses: $/...` (rhysd/actionlint
    // issue 711), which `release.yml` of `ci-publish` uses; docs/botica-release.md has the same flag.
    let args = [
        "-ignore",
        r#"reusable workflow call "\$/.+" at "uses" is not following the format"#,
    ];
    let Some(failed) = failures("actionlint", &args, "workflows-actionlint") else {
        return;
    };
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn the_workflows_of_the_recipes_pass_zizmor() {
    let args = [
        "--offline",
        "--no-progress",
        "--config",
        ".github/zizmor.yml",
    ];
    let Some(failed) = failures("zizmor", &args, "workflows-zizmor") else {
        return;
    };
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
