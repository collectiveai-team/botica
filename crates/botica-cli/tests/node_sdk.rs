//! The TypeScript SDK, `@botica/sdk` of `sdk/node`: an extension script that uses it answers
//! the protocol, ends after `shutdown`, and `botica lint` runs it as a rule of a project. The tests
//! run a real `node`.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{project_in, stderr, stdout, write};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// A rule written with the SDK: it flags each `x` of a line that starts with `x`.
fn script(sdk: &Path) -> String {
    format!(
        r#"import {{ byteOffsets, rule, serve }} from "file://{}";

rule(
  "acme/no-leading-x",
  ["python"],
  (path, text) => {{
    const toByte = byteOffsets(text);
    const found = [];
    let offset = 0;
    for (const line of text.split(/(?<=\n)/)) {{
      if (line.startsWith("x")) {{
        found.push({{ start: toByte(offset), end: toByte(offset + 1), message: "a line starts with x" }});
      }}
      offset += line.length;
    }}
    return found;
  }},
  {{ severity: "error" }},
);

{JS_ANYTHING}
serve();
"#,
        sdk.display()
    )
}

/// A second rule of the script, which no `rule.toml` of the catalog runs.
const JS_ANYTHING: &str =
    "rule(\"acme/js-anything\", [\"javascript\"], (path, text) => [{ start: 0, end: 1, message: \"js\" }]);\n";

/// Three rules: one that reports its option `limit`, one that reports every file, and one of
/// project scope that reports the first path with the number of paths and the root.
fn options_script(sdk: &Path) -> String {
    format!(
        r#"import {{ rule, serve }} from "file://{}";

rule("acme/limit", ["python"], (path, text, options) => [
  {{ start: 0, end: 1, message: `limit ${{options.limit ?? "none"}}` }},
]);
rule("acme/other", ["python"], () => [{{ start: 0, end: 1, message: "other" }}]);
rule(
  "acme/whole",
  ["python"],
  (root, paths, options) => [
    {{ path: paths[0], start: 0, end: 1, message: `${{paths.length}} files in ${{root}}, limit ${{options.limit}}` }},
  ],
  {{ scope: "project" }},
);

serve();
"#,
        sdk.display()
    )
}

#[test]
fn lint_files_runs_the_rules_it_names_with_their_options() {
    if !common::node::installed() {
        return;
    }
    let dir = lay_out("options");
    write(&dir, "options.mjs", &options_script(&sdk_entry()));
    let files = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"x\n"}],"rules":["acme/limit"],"options":{"acme/limit":{"limit":4}}}}"#;

    let replies = talk(&dir.join("options.mjs"), &[files, SHUTDOWN]);

    assert_eq!(
        replies[0],
        r#"{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[{"path":"a.py","rule":"acme/limit","severity":"warning","start":0,"end":1,"message":"limit 4"}]}}"#
    );
}

#[test]
fn a_rule_of_project_scope_answers_lint_project_with_paths_of_its_own() {
    if !common::node::installed() {
        return;
    }
    let dir = lay_out("project");
    write(&dir, "options.mjs", &options_script(&sdk_entry()));
    let project = r#"{"jsonrpc":"2.0","id":2,"method":"lint/project","params":{"root":"/repo","files":["a.py","b.py"],"rules":["acme/whole"],"options":{"acme/whole":{"limit":2}}}}"#;

    let replies = talk(&dir.join("options.mjs"), &[project, SHUTDOWN]);

    assert_eq!(
        replies[0],
        r#"{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[{"path":"a.py","rule":"acme/whole","severity":"warning","start":0,"end":1,"message":"2 files in /repo, limit 2"}]}}"#
    );
}

fn sdk_entry() -> PathBuf {
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sdk/node/src/index.js"))
        .expect("the SDK is in sdk/node")
}

/// Runs the script with `node` and gives it the requests while its stdin stays open: the replies
/// are one line for each request, and the script has to end by itself after the last.
fn talk(script: &Path, requests: &[&str]) -> Vec<String> {
    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start node");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let mut replies = Vec::new();
    for request in requests {
        writeln!(stdin, "{request}").expect("write the request");
        let mut reply = String::new();
        stdout.read_line(&mut reply).expect("read the reply");
        replies.push(reply.trim_end().to_owned());
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let ended = loop {
        if child.try_wait().expect("wait for node").is_some() {
            break true;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            break false;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(ended, "node did not end after `shutdown`: {replies:?}");
    replies
}

fn lay_out(name: &str) -> PathBuf {
    let dir = project_in("node-sdk", name, Some(""));
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
         runtime = \"node\"\nlanguages = [\"python\"]\nfile = \"no-leading-x.mjs\"\n",
    );
    write(&dir, &format!("{rule}/README.md"), README);
    write(
        &dir,
        &format!("{rule}/no-leading-x.mjs"),
        &script(&sdk_entry()),
    );
    dir
}

const SHUTDOWN: &str = r#"{"jsonrpc":"2.0","id":3,"method":"shutdown"}"#;

#[test]
fn a_script_with_the_sdk_answers_the_protocol_and_ends_after_shutdown() {
    if !common::node::installed() {
        return;
    }
    let dir = lay_out("protocol");
    let script = dir.join(".botica/rules/acme/structure/no-leading-x/no-leading-x.mjs");
    let files = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"ok = 1\nx = 2\n"},{"path":"b.js","text":"x\n"}]}}"#;

    let replies = talk(
        &script,
        &[
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"client":"botica"}}"#,
            files,
            SHUTDOWN,
        ],
    );

    assert_eq!(
        replies,
        [
            r#"{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"rules":[{"id":"acme/no-leading-x","languages":["python"]},{"id":"acme/js-anything","languages":["javascript"]}]}}"#,
            r#"{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[{"path":"a.py","rule":"acme/no-leading-x","severity":"error","start":7,"end":8,"message":"a line starts with x"},{"path":"b.js","rule":"acme/js-anything","severity":"warning","start":0,"end":1,"message":"js"}]}}"#,
            r#"{"jsonrpc":"2.0","id":3,"result":null}"#,
        ]
    );
}

#[test]
fn offsets_are_bytes_and_not_characters() {
    if !common::node::installed() {
        return;
    }
    let dir = lay_out("bytes");
    let script = dir.join(".botica/rules/acme/structure/no-leading-x/no-leading-x.mjs");
    let files = r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"é = 1\nx = 2\n"}]}}"#;

    let replies = talk(&script, &[files, SHUTDOWN]);

    assert!(
        replies[0].contains(r#""start":7,"end":8"#),
        "{}",
        replies[0]
    );
}

#[test]
fn an_unknown_method_gets_an_error_and_the_script_goes_on() {
    if !common::node::installed() {
        return;
    }
    let dir = lay_out("unknown");
    let script = dir.join(".botica/rules/acme/structure/no-leading-x/no-leading-x.mjs");

    let replies = talk(
        &script,
        &[r#"{"jsonrpc":"2.0","id":7,"method":"nope"}"#, SHUTDOWN],
    );

    assert_eq!(
        replies,
        [
            r#"{"jsonrpc":"2.0","id":7,"error":{"code":-32601,"message":"nope"}}"#,
            r#"{"jsonrpc":"2.0","id":3,"result":null}"#,
        ]
    );
}

#[test]
fn botica_lint_runs_a_rule_written_with_the_sdk() {
    if !common::node::installed() {
        return;
    }
    let dir = lay_out("lint");
    let path = ".botica/rules/acme/structure/no-leading-x/no-leading-x.mjs";
    let only_its_rule = script(&sdk_entry()).replace(JS_ANYTHING, "");
    write(&dir, path, &only_its_rule);
    write(&dir, "app.py", "ok = 1\nx = 2\n");

    let output = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["lint", "."])
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
