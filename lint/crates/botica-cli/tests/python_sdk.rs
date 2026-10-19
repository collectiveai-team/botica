//! The Python SDK, `botica.sdk` of the wheel `botica`: an extension script that uses it answers the
//! protocol, and `botica lint` runs it as a rule of a project.

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{project_in, stderr, stdout, write};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// A rule written with the SDK: it flags each `x` of a line that starts with `x`.
const SCRIPT: &str = r#"from botica.sdk import char_to_byte_offsets, rule, serve


@rule("acme/no-leading-x", ["python"], severity="error")
def no_leading_x(path, text, options):
    to_byte = char_to_byte_offsets(text)
    offset = 0
    for line in text.splitlines(keepends=True):
        if line.startswith("x"):
            yield (to_byte(offset), to_byte(offset + 1), "a line starts with x")
        offset += len(line)


if __name__ == "__main__":
    serve()
"#;

fn sdk_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../python")
}

/// Runs the script with `python3`, the way `uv run --script` would, and gives it the requests.
fn talk(script: &Path, requests: &[&str]) -> Vec<String> {
    let mut child = Command::new("python3")
        .arg(script)
        .env("PYTHONPATH", sdk_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start python3");
    let mut stdin = child.stdin.take().expect("stdin");
    for request in requests {
        writeln!(stdin, "{request}").expect("write the request");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("the script ends");
    String::from_utf8(output.stdout)
        .expect("UTF-8")
        .lines()
        .map(str::to_owned)
        .collect()
}

fn lay_out(name: &str) -> PathBuf {
    let dir = project_in("python-sdk", name, Some(""));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    let root = ".botica/rules/acme";
    write(&dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        &dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{root}/structure/no-leading-x");
    write(
        &dir,
        &format!("{rule}/rule.toml"),
        "id = \"acme/no-leading-x\"\ncode = \"ACMESTRUC001\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         scope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\ndefault-enabled = true\n\
         default-severity = \"error\"\n\n[[implementations]]\nkind = \"extension\"\n\
         runtime = \"python\"\nlanguages = [\"python\"]\nfile = \"no-leading-x.py\"\n",
    );
    write(&dir, &format!("{rule}/README.md"), README);
    write(&dir, &format!("{rule}/no-leading-x.py"), SCRIPT);
    dir
}

#[test]
fn a_script_with_the_sdk_answers_the_protocol() {
    let dir = lay_out("protocol");
    let script = dir.join(".botica/rules/acme/structure/no-leading-x/no-leading-x.py");
    let files = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"ok = 1\nx = 2\n"},{"path":"b.js","text":"x\n"}]}}"#;

    let replies = talk(
        &script,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"client":"botica"}}"#,
            files,
            r#"{"jsonrpc":"2.0","id":3,"method":"shutdown"}"#,
        ],
    );

    assert_eq!(
        replies,
        [
            r#"{"jsonrpc": "2.0", "id": 1, "result": {"protocolVersion": 1, "rules": [{"id": "acme/no-leading-x", "languages": ["python"]}]}}"#,
            r#"{"jsonrpc": "2.0", "id": 2, "result": {"diagnostics": [{"path": "a.py", "rule": "acme/no-leading-x", "severity": "error", "start": 7, "end": 8, "message": "a line starts with x"}]}}"#,
            r#"{"jsonrpc": "2.0", "id": 3, "result": null}"#,
        ]
    );
}

/// Three rules: one that reports its option `limit`, one that reports every file, and one of
/// project scope that reports the first path with the number of paths and the root.
const OPTIONS_SCRIPT: &str = r#"from botica.sdk import rule, serve


@rule("acme/limit", ["python"])
def limit(path, text, options):
    yield (0, 1, f"limit {options.get('limit', 'none')}")


@rule("acme/other", ["python"])
def other(path, text, options):
    yield (0, 1, "other")


@rule("acme/whole", ["python"], scope="project")
def whole(root, paths, options):
    yield (paths[0], 0, 1, f"{len(paths)} files in {root}, limit {options.get('limit')}")


if __name__ == "__main__":
    serve()
"#;

fn options_script(name: &str) -> PathBuf {
    let dir = lay_out(name);
    let script = dir.join("options.py");
    write(&dir, "options.py", OPTIONS_SCRIPT);
    script
}

#[test]
fn lint_files_runs_the_rules_it_names_with_their_options() {
    let script = options_script("options");
    let files = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"x\n"}],"rules":["acme/limit"],"options":{"acme/limit":{"limit":4}}}}"#;

    let replies = talk(&script, &[files]);

    assert_eq!(
        replies,
        [
            r#"{"jsonrpc": "2.0", "id": 2, "result": {"diagnostics": [{"path": "a.py", "rule": "acme/limit", "severity": "warning", "start": 0, "end": 1, "message": "limit 4"}]}}"#
        ]
    );
}

#[test]
fn a_rule_of_project_scope_answers_lint_project_with_paths_of_its_own() {
    let script = options_script("project");
    let project = r#"{"jsonrpc":"2.0","id":2,"method":"lint/project","params":{"root":"/repo","files":["a.py","b.py"],"rules":["acme/whole"],"options":{"acme/whole":{"limit":2}}}}"#;

    let replies = talk(&script, &[project]);

    assert_eq!(
        replies,
        [
            r#"{"jsonrpc": "2.0", "id": 2, "result": {"diagnostics": [{"path": "a.py", "rule": "acme/whole", "severity": "warning", "start": 0, "end": 1, "message": "2 files in /repo, limit 2"}]}}"#
        ]
    );
}

#[test]
fn offsets_are_bytes_and_not_characters() {
    let dir = lay_out("bytes");
    let script = dir.join(".botica/rules/acme/structure/no-leading-x/no-leading-x.py");
    let text = "é = 1\\nx = 2\\n";
    let files = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{{"files":[{{"path":"a.py","text":"{text}"}}]}}}}"#
    );

    let replies = talk(
        &script,
        &[&files, r#"{"jsonrpc":"2.0","id":3,"method":"shutdown"}"#],
    );

    assert!(
        replies[0].contains(r#""start": 7, "end": 8"#),
        "{}",
        replies[0]
    );
}

#[test]
fn botica_lint_runs_a_rule_written_with_the_sdk() {
    let dir = lay_out("lint");
    let bin = dir.join("fake-bin");
    write(
        &bin,
        "uv",
        "#!/bin/sh\n[ \"$1 $2\" = \"run --script\" ] || exit 9\nexec python3 \"$3\"\n",
    );
    let mut permissions = std::fs::metadata(bin.join("uv"))
        .expect("exists")
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    std::fs::set_permissions(bin.join("uv"), permissions).expect("executable");
    write(&dir, "app.py", "ok = 1\nx = 2\n");

    let output = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["lint", "."])
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("PYTHONPATH", sdk_dir())
        .current_dir(&dir)
        .output()
        .expect("run botica");

    assert_eq!(
        stdout(&output),
        "app.py:2:1: error [acme/no-leading-x] a line starts with x\n",
        "{}",
        stderr(&output)
    );
}
