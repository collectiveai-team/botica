//! `botica server`: an editor opens a buffer that is not on disk and gets the findings of the
//! rules of the project.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use common::{project_in, write};
use serde_json::{json, Value};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

fn lay_out_acme(dir: &std::path::Path) {
    let root = ".botica/rules/acme";
    write(dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{root}/structure/no-eval");
    write(
        dir,
        &format!("{rule}/rule.toml"),
        "id = \"acme/no-eval\"\ncode = \"ACMESTRUC001\"\nsummary = \"Do not call eval\"\n\
         status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\n\
         default-enabled = true\ndefault-severity = \"error\"\n\n[[implementations]]\n\
         kind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n",
    );
    write(dir, &format!("{rule}/README.md"), README);
    write(
        dir,
        &format!("{rule}/python.yml"),
        "id: acme/no-eval\nlanguage: python\nseverity: error\n\
         message: \"ACMESTRUC001 (acme/no-eval): do not call eval\"\n\
         rule:\n  pattern: eval($$$ARGS)\n",
    );
}

fn frame(message: &Value) -> String {
    let body = message.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

fn read_frame(output: &mut impl BufRead) -> Value {
    let mut length = 0;
    loop {
        let mut line = String::new();
        output.read_line(&mut line).expect("a header line");
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        length = line
            .strip_prefix("Content-Length: ")
            .map_or(length, |value| value.parse().expect("a number"));
    }
    let mut body = vec![0; length];
    output.read_exact(&mut body).expect("the body");
    serde_json::from_slice(&body).expect("JSON")
}

#[test]
fn a_buffer_that_is_not_on_disk_gets_the_findings_of_the_rules_of_the_project() {
    let dir = project_in("server", "buffer", Some(""));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    lay_out_acme(&dir);
    let uri = format!("file://{}/scratch.py", dir.display());

    let mut child = Command::new(env!("CARGO_BIN_EXE_botica"))
        .arg("server")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start botica server");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));

    stdin
        .write_all(
            frame(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}))
                .as_bytes(),
        )
        .expect("initialize");
    let initialized = read_frame(&mut stdout);
    stdin
        .write_all(
            frame(
                &json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
                "textDocument": {"uri": uri, "languageId": "python", "version": 1,
                                 "text": "x = 1\neval(\"1\")\n"}}}),
            )
            .as_bytes(),
        )
        .expect("didOpen");
    let published = read_frame(&mut stdout);
    stdin
        .write_all(frame(&json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"})).as_bytes())
        .expect("shutdown");
    let shut_down = read_frame(&mut stdout);
    stdin
        .write_all(frame(&json!({"jsonrpc": "2.0", "method": "exit"})).as_bytes())
        .expect("exit");
    let status = child.wait().expect("botica ends");

    assert_eq!(initialized["result"]["serverInfo"]["name"], "botica");
    assert_eq!(published["method"], "textDocument/publishDiagnostics");
    assert_eq!(published["params"]["uri"], uri);
    let found = &published["params"]["diagnostics"][0];
    assert_eq!(found["code"], "acme/no-eval");
    assert_eq!(found["severity"], 1);
    assert_eq!(found["range"]["start"], json!({"line": 1, "character": 0}));
    assert_eq!(
        published["params"]["diagnostics"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(shut_down["result"], Value::Null);
    assert!(status.success());
}
