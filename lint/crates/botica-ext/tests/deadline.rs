//! What `ExtensionHost` does with a process that does not answer in time.

use botica_ext::{Deadlines, ExtensionHost, FileIn};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const INITIALIZED: &str = r#"read -r line
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"rules":[]}}'"#;

fn deadlines(seconds: u64) -> Deadlines {
    Deadlines {
        initialize: Duration::from_secs(seconds),
        request: Duration::from_secs(seconds),
    }
}

/// A host for a shell script that notes its pid in `<name>.pid` and then runs `body`.
fn host(name: &str, body: &str, deadlines: Deadlines) -> (ExtensionHost, PathBuf) {
    let dir = std::env::temp_dir().join(format!("botica-ext-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a directory");
    let pid = dir.join("pid");
    let script = format!("echo $$ > '{}'\n{body}\n", pid.display());
    let command = ["sh".to_owned(), "-c".to_owned(), script];
    let host = ExtensionHost::spawn_with(&command, None, deadlines).expect("the host starts");
    (host, pid)
}

fn is_running(pid_file: &Path) -> bool {
    let pid = std::fs::read_to_string(pid_file).expect("the process started");
    Path::new("/proc").join(pid.trim()).exists()
}

#[test]
fn a_process_that_does_not_answer_lint_files_is_gone_when_the_error_returns() {
    let (mut host, pid) = host(
        "gone",
        &format!("{INITIALIZED}\nread -r line\nexec sleep 600"),
        deadlines(1),
    );

    let error = host
        .lint_files(&[], None)
        .expect_err("no answer")
        .to_string();

    assert_eq!(
        error,
        "extension did not answer `lint/files` within 1s and was killed"
    );
    assert!(!is_running(&pid), "the process is still running");
}

#[test]
fn a_process_that_does_not_read_its_requests_is_killed_at_the_deadline() {
    let (mut host, _) = host(
        "unread",
        &format!("{INITIALIZED}\nexec sleep 600"),
        deadlines(1),
    );
    let text = "x".repeat(4 * 1024 * 1024);
    let started = Instant::now();

    let error = host
        .lint_files(
            &[FileIn {
                path: "a.py",
                text: &text,
            }],
            None,
        )
        .expect_err("no answer")
        .to_string();

    assert!(error.contains("within 1s"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn a_process_that_answers_in_time_is_not_touched() {
    let (mut host, pid) = host(
        "answers",
        &format!(
            "{INITIALIZED}\nread -r line\nprintf '%s\\n' '{}'\nexec sleep 600",
            r#"{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[]}}"#
        ),
        deadlines(30),
    );

    let found = host.lint_files(&[], None).expect("answered");

    assert!(found.is_empty());
    assert!(is_running(&pid));
}
