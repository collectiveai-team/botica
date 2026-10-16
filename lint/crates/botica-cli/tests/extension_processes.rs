//! How `botica lint` starts the extension rules of a project and what it does when one fails.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use common::extension::{
    add_rule, findings, folder, implementation, lint, project, rule_toml, script, README,
};
use common::{stderr, stdout, write};

#[test]
fn a_process_that_fails_is_a_failure_of_the_run_and_the_other_findings_stay() {
    let dir = project("failing", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    add_rule(&dir, "other", "ACMESTRUC002", "", "acme/other");
    write(&dir, &format!("{}/ext.sh", folder("by-sh")), "exit 3\n");

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(findings(&output), ["app.py acme/other"]);
    assert!(
        stderr(&output).contains("acme/by-sh"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_process_that_does_not_end_after_shutdown_is_killed_and_is_a_failure_of_the_run() {
    let dir = project("lingering", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    add_rule(&dir, "other", "ACMESTRUC002", "", "acme/other");
    let lingering = script("acme/by-sh", "error").replace("exit 0 ;;", "exec sleep 600 ;;");
    write(&dir, &format!("{}/ext.sh", folder("by-sh")), &lingering);

    let mut child = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["lint", "."])
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run botica");
    let deadline = Instant::now() + Duration::from_secs(60);
    while child.try_wait().expect("wait for botica").is_none() {
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("botica lint is still waiting for a process that does not end");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let output = child.wait_with_output().expect("the output");

    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert_eq!(
        findings(&output),
        ["app.py acme/by-sh", "app.py acme/other"]
    );
    assert!(
        stderr(&output).contains("acme/by-sh") && stderr(&output).contains("shutdown"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_program_that_does_not_exist_is_a_failure_of_the_run() {
    let dir = project("no-program", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    let toml = rule_toml(
        "by-sh",
        "ACMESTRUC001",
        "",
        &implementation(
            "command",
            "\"python\"",
            "command = [\"no-such-program-here\"]",
        ),
    );
    write(&dir, &format!("{}/rule.toml", folder("by-sh")), &toml);

    let output = lint(&dir, &[]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("no-such-program-here"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn comments_silence_an_extension_finding_and_are_never_reported_as_unused() {
    let dir = project("silenced", "");
    add_rule(&dir, "by-sh", "ACMESTRUC001", "", "acme/by-sh");
    write(&dir, "app.py", "x = 1  # botica: ignore[acme/by-sh]\n");
    write(&dir, "b.py", "x = 1  # noqa: ACMESTRUC001\n");
    write(
        &dir,
        "c.py",
        "x = 1  # botica: ignore[acme/by-sh]\n\ny = 2\n",
    );

    let output = lint(&dir, &[]);

    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
}

/// A directory of fake `uv` and `node` that run the script they are given with `sh`, noting what
/// they were asked in `runs.log`.
fn fake_tools(dir: &Path) -> PathBuf {
    let bin = dir.join("fake-bin");
    write(
        &bin,
        "uv",
        "#!/bin/sh\necho \"uv $*\" >> \"$(dirname \"$0\")/../runs.log\"\n[ \"$1 $2\" = \"run --script\" ] || exit 9\nexec sh \"$3\"\n",
    );
    write(
        &bin,
        "node",
        "#!/bin/sh\necho \"node $*\" >> \"$(dirname \"$0\")/../runs.log\"\nexec sh \"$1\"\n",
    );
    for tool in ["uv", "node"] {
        let path = bin.join(tool);
        let mut permissions = std::fs::metadata(&path).expect("exists").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        std::fs::set_permissions(&path, permissions).expect("executable");
    }
    bin
}

fn lint_with_tools(dir: &Path, bin: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["lint", "."])
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .current_dir(dir)
        .output()
        .expect("run botica")
}

#[test]
fn a_python_rule_runs_with_uv_and_a_node_rule_with_node() {
    let dir = project("runtimes", "");
    let bin = fake_tools(&dir);
    for (slug, code, runtime, file) in [
        ("by-py", "ACMESTRUC001", "python", "ext.py"),
        ("by-js", "ACMESTRUC002", "node", "ext.mjs"),
    ] {
        let root = folder(slug);
        let how = format!("file = \"{file}\"");
        write(
            &dir,
            &format!("{root}/rule.toml"),
            &rule_toml(slug, code, "", &implementation(runtime, "\"python\"", &how)),
        );
        write(&dir, &format!("{root}/README.md"), README);
        write(
            &dir,
            &format!("{root}/{file}"),
            &script(&format!("acme/{slug}"), "warning"),
        );
    }

    let output = lint_with_tools(&dir, &bin);

    assert_eq!(
        findings(&output),
        ["app.py acme/by-js", "app.py acme/by-py"],
        "{}",
        stderr(&output)
    );
    let runs = std::fs::read_to_string(dir.join("runs.log")).expect("the tools ran");
    let root = dir.display();
    let mut lines: Vec<&str> = runs.lines().collect();
    lines.sort_unstable();
    assert_eq!(
        lines,
        [
            format!("node {root}/{}/ext.mjs", folder("by-js")),
            format!("uv run --script {root}/{}/ext.py", folder("by-py")),
        ]
    );
}
