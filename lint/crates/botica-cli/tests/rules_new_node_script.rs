//! The script that `botica rules new --runtime node` writes, run by a real `node`.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::{command, project_in, stderr, write};

const CATALOG: &str = ".botica/rules/acme";

fn node_is_installed() -> bool {
    Command::new("node")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn the_node_script_answers_the_protocol_and_exits_when_it_is_asked_to_shut_down() {
    if !node_is_installed() {
        eprintln!("node is not installed: skipped");
        return;
    }
    let dir = project_in("rules-new-node-script", "exits", Some(""));
    write(
        &dir,
        &format!("{CATALOG}/catalog.toml"),
        "code = \"ACME\"\n",
    );
    write(
        &dir,
        &format!("{CATALOG}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let created = command(
        &dir,
        &[
            "rules",
            "new",
            "structure/no-blobs",
            "--kind",
            "extension",
            "--runtime",
            "node",
            "--language",
            "python",
            "--catalog",
            CATALOG,
        ],
    );
    assert!(created.status.success(), "{}", stderr(&created));
    let script = dir.join(format!("{CATALOG}/structure/no-blobs/no-blobs.mjs"));

    let mut child = Command::new("node")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start node");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let mut ask = |request: &str| {
        writeln!(stdin, "{request}").expect("write the request");
        let mut reply = String::new();
        stdout.read_line(&mut reply).expect("read the reply");
        reply
    };

    let initialized = ask(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    let linted = ask(
        r#"{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"x = 1\n"}]}}"#,
    );
    let stopped = ask(r#"{"jsonrpc":"2.0","id":3,"method":"shutdown","params":null}"#);

    assert!(initialized.contains("acme/no-blobs"), "{initialized}");
    assert!(linted.contains(r#""diagnostics":[]"#), "{linted}");
    assert!(stopped.contains(r#""id":3"#), "{stopped}");
    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for node") {
            break Some(status);
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert!(
        status.is_some_and(|status| status.success()),
        "node did not exit after `shutdown` while its stdin was open"
    );
}
