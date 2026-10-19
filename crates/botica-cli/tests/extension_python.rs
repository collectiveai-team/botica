//! Which Python runs the scripts of the extension rules (`[extensions] python`), and
//! `botica ext sync`, which prepares their environments beforehand.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use common::extension::{add_rule, folder, implementation, project, rule_toml, README};
use common::{stderr, stdout, write};

/// Adds `acme/py-rule`, whose Python script is `check.py`.
fn add_python_rule(dir: &Path) {
    let root = folder("py-rule");
    write(
        dir,
        &format!("{root}/rule.toml"),
        &rule_toml(
            "py-rule",
            "ACMESTRUC002",
            "",
            &implementation("python", "\"python\"", "file = \"check.py\""),
        ),
    );
    write(dir, &format!("{root}/README.md"), README);
    write(dir, &format!("{root}/check.py"), "");
}

fn python_project(name: &str, config: &str) -> PathBuf {
    let dir = project(name, config);
    add_python_rule(&dir);
    dir
}

/// What `botica lint` would run for `acme/py-rule`, from `botica rules --json`.
fn runs(dir: &Path) -> serde_json::Value {
    let output = common::command(dir, &["rules", "--json"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).expect("JSON");
    document["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .find(|rule| rule["rule"] == "acme/py-rule")
        .expect("acme/py-rule")["implementations"][0]["runs"]
        .clone()
}

fn uv_with(python: &[&str]) -> serde_json::Value {
    let script = format!("{}/check.py", folder("py-rule"));
    let mut argv = vec!["uv", "run"];
    argv.extend_from_slice(python);
    argv.extend(["--script", &script]);
    serde_json::json!(argv)
}

#[test]
fn a_project_that_names_no_python_gets_the_python_of_botica() {
    let dir = python_project("python-default", "");

    assert_eq!(runs(&dir), uv_with(&["--python", "3.13"]));
}

#[test]
fn the_python_of_the_project_comes_from_python_version_then_the_venv_then_requires_python() {
    let dir = python_project("python-project", "");
    write(
        &dir,
        "pyproject.toml",
        "[project]\nname = \"a\"\nrequires-python = \">=3.10\"\n",
    );
    assert_eq!(runs(&dir), uv_with(&["--python", ">=3.10"]));

    write(
        &dir,
        ".venv/pyvenv.cfg",
        "home = /usr/bin\nimplementation = CPython\nversion_info = 3.11.4\n",
    );
    assert_eq!(runs(&dir), uv_with(&["--python", "3.11"]));

    write(&dir, ".python-version", "# pinned\n3.12\n");
    assert_eq!(runs(&dir), uv_with(&["--python", "3.12"]));
}

#[test]
fn the_configuration_names_a_version_or_the_python_of_the_system() {
    let dir = python_project("python-version", "[extensions]\npython = \"3.12\"\n");
    write(&dir, ".python-version", "3.11\n");
    assert_eq!(runs(&dir), uv_with(&["--python", "3.12"]));

    let dir = python_project("python-system", "[extensions]\npython = \"system\"\n");
    assert_eq!(runs(&dir), uv_with(&["--python-preference", "only-system"]));
}

#[test]
fn a_free_threaded_python_is_refused() {
    for (name, config, pin) in [
        ("python-ft-pin", "", Some("3.13t\n")),
        (
            "python-ft-config",
            "[extensions]\npython = \"3.14+freethreaded\"\n",
            None,
        ),
    ] {
        let dir = python_project(name, config);
        if let Some(pin) = pin {
            write(&dir, ".python-version", pin);
        }

        let output = common::command(&dir, &["lint", "."]);

        assert_eq!(output.status.code(), Some(2), "{name}: {}", stdout(&output));
        assert!(
            stderr(&output).contains("free-threaded"),
            "{name}: {}",
            stderr(&output)
        );
    }
}

/// A fake `uv` that writes its arguments to `uv.log` and exits with `status`.
fn fake_uv(dir: &Path, status: i32) -> PathBuf {
    let bin = dir.join("fake-bin");
    write(
        &bin,
        "uv",
        &format!(
            "#!/bin/sh\necho \"$@\" >> \"{}/uv.log\"\nexit {status}\n",
            dir.display()
        ),
    );
    let mut permissions = std::fs::metadata(bin.join("uv"))
        .expect("exists")
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    std::fs::set_permissions(bin.join("uv"), permissions).expect("executable");
    bin
}

fn sync(dir: &Path, bin: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["ext", "sync"])
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .current_dir(dir)
        .output()
        .expect("run botica")
}

#[test]
fn ext_sync_prepares_the_environment_of_each_python_script_with_its_python() {
    let dir = python_project("sync", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    write(&dir, ".python-version", "3.12\n");
    let bin = fake_uv(&dir, 0);

    let output = sync(&dir, &bin);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let log = std::fs::read_to_string(dir.join("uv.log")).expect("uv ran");
    assert_eq!(
        log,
        format!(
            "sync --python 3.12 --script {}/{}/check.py\n",
            dir.display(),
            folder("py-rule")
        )
    );
    assert_eq!(
        stdout(&output),
        "acme/by-sh: nothing to prepare\nacme/py-rule: ready\n"
    );
}

#[test]
fn ext_sync_fails_when_an_environment_cannot_be_prepared() {
    let dir = python_project("sync-fails", "");
    let bin = fake_uv(&dir, 1);

    let output = sync(&dir, &bin);

    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert!(
        stdout(&output).contains("acme/py-rule: failed"),
        "{}",
        stdout(&output)
    );
}
