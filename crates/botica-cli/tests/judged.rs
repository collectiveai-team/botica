use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("judged")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the working directory");
    dir
}

fn botica(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["lint"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run botica")
}

const NO_UTILS: &str = "{\"version\": \"2.1.0\", \"runs\": [{\"results\": [{\"ruleId\": \
    \"no-utils\", \"message\": {\"text\": \"m\"}, \"locations\": [{\"physicalLocation\": \
    {\"artifactLocation\": {\"uri\": \"utils.py\"}}}]}]}]}";

#[test]
fn a_finding_of_a_deterministic_rule_is_left_out_with_a_warning_and_nothing_is_linted() {
    let dir = workdir("deterministic");
    fs::write(dir.join("judge.sarif"), NO_UTILS).expect("write the SARIF");
    fs::write(dir.join("utils.py"), "x = 1\n").expect("write a file botica would flag");

    let output = botica(&dir, &["--judged", "judge.sarif"]);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "botica: `no-utils` is not a judgment rule of the catalog: its finding is left out\n"
    );
}

#[test]
fn a_sarif_that_cannot_be_read_or_parsed_fails_the_run() {
    let dir = workdir("broken");
    fs::write(dir.join("broken.sarif"), "not json").expect("write the SARIF");

    for (file, expected) in [
        ("missing.sarif", "cannot read `missing.sarif`"),
        ("broken.sarif", "the judged findings are not SARIF"),
    ] {
        let output = botica(&dir, &["--judged", file]);

        assert_eq!(output.status.code(), Some(2), "{file}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected), "{file}: {stderr}");
    }
}

#[test]
fn judged_findings_take_no_paths_and_no_extension() {
    let dir = workdir("conflicts");
    fs::write(dir.join("judge.sarif"), NO_UTILS).expect("write the SARIF");

    for args in [
        ["--judged", "judge.sarif", "src"].as_slice(),
        ["--judged", "judge.sarif", "--ext", "x"].as_slice(),
    ] {
        assert_eq!(botica(&dir, args).status.code(), Some(2), "{args:?}");
    }
}
