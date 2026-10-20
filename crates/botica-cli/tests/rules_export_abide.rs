//! `botica rules export --format abide`: the rubric of abide from the catalog and the project.
//! The project of these tests selects no judgment rule, so its rubric has none; the unit tests of
//! `rubric` cover the rules.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::{json, Value};

const AGENTS_MD: &str = "notes\n<!-- botica:rules -->\n<!-- /botica:rules -->\n";

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-export-abide")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("create the working directory");
    fs::write(dir.join("AGENTS.md"), AGENTS_MD).expect("write AGENTS.md");
    dir
}

fn export(dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["rules", "export", "--format", "abide"])
        .args(extra)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn rubric(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("the rubric")).expect("JSON")
}

#[test]
fn the_rubric_goes_to_where_abide_reads_it_and_abide_never_recompiles_it() {
    let dir = workdir("default");

    let output = export(&dir, &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    let rubric = rubric(&dir.join(".abide/rubric.json"));
    assert_eq!(rubric["version"], 1);
    assert_eq!(rubric["manual"], true);
    assert_eq!(rubric["sources"], json!([{ "path": "AGENTS.md" }]));
    assert_eq!(rubric["rules"], json!([]));
    let compiled_at = rubric["compiledAt"].as_str().expect("compiledAt");
    assert!(
        compiled_at.len() == 20 && compiled_at.starts_with("20") && compiled_at.ends_with('Z'),
        "{compiled_at}"
    );
}

#[test]
fn output_and_agents_md_name_other_files() {
    let dir = workdir("paths");
    fs::create_dir_all(dir.join("docs")).expect("docs");
    fs::rename(dir.join("AGENTS.md"), dir.join("docs/AGENTS.md")).expect("move AGENTS.md");

    let output = export(
        &dir,
        &[
            "--agents-md",
            "docs/AGENTS.md",
            "--output",
            "out/rubric.json",
        ],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        rubric(&dir.join("out/rubric.json"))["sources"],
        json!([{ "path": "docs/AGENTS.md" }])
    );
}

#[test]
fn without_agents_md_there_is_no_rubric() {
    let dir = workdir("no-agents-md");
    fs::remove_file(dir.join("AGENTS.md")).expect("remove AGENTS.md");

    let output = export(&dir, &[]);

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("cannot read `AGENTS.md`"),
        "{}",
        stderr(&output)
    );
    assert!(!dir.join(".abide").exists());
}

#[test]
fn a_rubric_with_the_same_sources_and_rules_is_left_as_it_is() {
    let dir = workdir("unchanged");
    assert!(export(&dir, &[]).status.success());
    let path = dir.join(".abide/rubric.json");
    let mut kept = rubric(&path);
    kept["compiledAt"] = json!("2020-01-01T00:00:00Z");
    fs::write(&path, kept.to_string()).expect("write the rubric");

    let output = export(&dir, &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(rubric(&path)["compiledAt"], "2020-01-01T00:00:00Z");
}

#[test]
fn check_passes_on_a_rubric_up_to_date_and_fails_on_one_that_differs_or_is_missing() {
    let dir = workdir("check");

    let missing = export(&dir, &["--check"]);
    assert!(!missing.status.success());
    assert!(!dir.join(".abide").exists());

    assert!(export(&dir, &[]).status.success());
    let fresh = export(&dir, &["--check"]);
    assert!(fresh.status.success(), "{}", stderr(&fresh));

    let path = dir.join(".abide/rubric.json");
    let mut stale = rubric(&path);
    stale["sources"] = json!([{ "path": "CLAUDE.md" }]);
    fs::write(&path, stale.to_string()).expect("write the rubric");

    let output = export(&dir, &["--check"]);

    assert!(!output.status.success());
    for output in [&missing, &output] {
        assert!(
            stderr(output).contains(
                "`.abide/rubric.json` differs from the catalog, the configuration or the \
                 `ignore-file` comments of the project: run `botica rules export --format abide`"
            ),
            "{}",
            stderr(output)
        );
    }
    assert_eq!(rubric(&path), stale);
}

#[test]
fn a_format_that_writes_a_directory_needs_output() {
    let dir = workdir("no-output");

    let output = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["rules", "export", "--format", "markdown"])
        .current_dir(&dir)
        .output()
        .expect("run botica");

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("--format markdown needs --output"),
        "{}",
        stderr(&output)
    );
}
