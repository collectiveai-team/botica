mod common;

use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

fn closed_pipe() -> Stdio {
    let (reader, writer) = io::pipe().expect("create pipe");
    drop(reader);
    writer.into()
}

fn run(dir: &Path, args: &[&str], stdout: Stdio, stderr: Stdio) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(args)
        .current_dir(dir)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .expect("run botica");
    let stderr = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            pipe.read_to_end(&mut bytes).expect("read stderr");
            bytes
        })
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().expect("poll botica").is_none() {
        if Instant::now() >= deadline {
            child.kill().expect("stop botica");
            let output = child.wait_with_output().expect("reap botica");
            panic!("botica timed out: {}", common::stderr(&output));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut output = child.wait_with_output().expect("collect output");
    if let Some(stderr) = stderr {
        output.stderr = stderr.join().expect("join stderr reader");
    }
    output
}

#[test]
fn listing_recipes_with_a_closed_consumer_succeeds_without_a_diagnostic() {
    let dir = common::project_in("broken-pipe", "recipe-list", None);

    let output = run(&dir, &["recipe", "list"], closed_pipe(), Stdio::piped());

    assert_eq!(output.status.code(), Some(0), "{}", common::stderr(&output));
    assert!(output.stderr.is_empty(), "{}", common::stderr(&output));
}

#[test]
fn closing_stdout_preserves_command_results_across_command_families() {
    let dir = common::project_in("broken-pipe", "families", None);
    common::recipes::add_recipe(
        &dir,
        "mine",
        "[[files]]\npath = \"first.txt\"\ntemplate = \"files/first.txt\"\n",
    );
    let commands: &[(&[&str], i32)] = &[
        (&["recipe", "show", "mine"], 0),
        (&["recipe", "check", "mine"], 1),
        (&["refs", "list"], 0),
        (&["refs", "show", "test-no-network"], 0),
        (&["rules"], 0),
        (&["rules", "--json"], 0),
        (&["explain", "file-size-guard"], 0),
        (&["--help"], 0),
        (&["--version"], 0),
        (&["recipe", "--help"], 0),
    ];
    for (args, code) in commands {
        let normal = common::command(&dir, args);
        assert_eq!(
            normal.status.code(),
            Some(*code),
            "{args:?}: {}",
            common::stderr(&normal)
        );
        assert!(!normal.stdout.is_empty(), "{args:?}");

        let output = run(&dir, args, closed_pipe(), Stdio::piped());

        assert_eq!(
            output.status.code(),
            Some(*code),
            "{args:?}: {}",
            common::stderr(&output)
        );
        assert_eq!(output.stderr, normal.stderr, "{args:?}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn stdout_errors_other_than_broken_pipe_fail_including_help_and_version() {
    let dir = common::project_in("broken-pipe", "full", None);
    common::write(&dir, "big.py", &common::lines(701));
    for args in [
        vec!["recipe", "list"],
        vec!["rules", "--json"],
        vec!["lint", "big.py", "--no-astgrep"],
        vec!["lint", "big.py", "--no-astgrep", "--output-format", "json"],
        vec!["--help"],
        vec!["--version"],
        vec!["recipe", "--help"],
    ] {
        let full = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("open full device");

        let output = run(&dir, &args, full.into(), Stdio::piped());

        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            common::stderr(&output)
        );
        assert!(
            common::stderr(&output).contains("No space left"),
            "{args:?}: {}",
            common::stderr(&output)
        );
    }
}

#[test]
fn lint_preserves_findings_and_exit_zero_when_its_report_is_discarded() {
    let dir = common::project_in(
        "broken-pipe",
        "lint",
        Some("select = [\"file-size-guard\"]\n"),
    );
    common::write(&dir, "big.py", &common::lines(701));
    for format in ["text", "json", "sarif", "github", "markdown"] {
        for (extra, code) in [
            (vec![], 1),
            (
                vec!["--statistics"],
                if matches!(format, "text" | "json") {
                    1
                } else {
                    2
                },
            ),
            (vec!["--exit-zero"], 0),
        ] {
            let mut args = vec!["lint", "big.py", "--no-astgrep", "--output-format", format];
            args.extend(extra);
            let normal = common::command(&dir, &args);
            assert_eq!(
                normal.status.code(),
                Some(code),
                "{args:?}: {}",
                common::stderr(&normal)
            );
            if code != 2 {
                assert!(!normal.stdout.is_empty(), "{args:?}");
            }

            let output = run(&dir, &args, closed_pipe(), Stdio::piped());

            assert_eq!(
                output.status.code(),
                Some(code),
                "{args:?}: {}",
                common::stderr(&output)
            );
            assert_eq!(output.stderr, normal.stderr, "{args:?}");
        }
    }
}

#[test]
fn closing_stdout_does_not_interrupt_installation_or_hide_pending_steps() {
    for pending in [false, true] {
        let dir = common::project_in("broken-pipe", &format!("install-{pending}"), None);
        common::recipes::add_recipe(
            &dir,
            "mine",
            "[[files]]\npath = \"first.txt\"\ntemplate = \"files/first.txt\"\n\n\
             [[files]]\npath = \"last.txt\"\ntemplate = \"files/last.txt\"\n",
        );
        if pending {
            common::write(&dir, "first.txt", "keep me\n");
        }

        let output = run(
            &dir,
            &["recipe", "install", "mine"],
            closed_pipe(),
            Stdio::piped(),
        );

        assert_eq!(
            output.status.code(),
            Some(i32::from(pending)),
            "{}",
            common::stderr(&output)
        );
        assert!(output.stderr.is_empty(), "{}", common::stderr(&output));
        assert_eq!(
            std::fs::read_to_string(dir.join("first.txt")).expect("first step"),
            if pending { "keep me\n" } else { "template\n" }
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("last.txt")).expect("last step"),
            "template\n"
        );
    }
}

#[test]
fn closed_diagnostic_streams_do_not_turn_failures_into_panics_or_successes() {
    let dir = common::project_in("broken-pipe", "stderr", None);
    for args in [
        vec!["recipe", "show", "does-not-exist"],
        vec!["--not-an-option"],
    ] {
        let normal = common::command(&dir, &args);
        assert_eq!(normal.status.code(), Some(2));
        assert!(!normal.stderr.is_empty());

        let output = run(&dir, &args, closed_pipe(), closed_pipe());

        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn a_real_installation_error_after_discarded_progress_still_fails() {
    let dir = common::project_in("broken-pipe", "install-error", None);
    common::recipes::add_recipe(
        &dir,
        "mine",
        "[[files]]\npath = \"first.txt\"\ntemplate = \"files/first.txt\"\n\n\
         [[files]]\npath = \"z-blocked/last.txt\"\ntemplate = \"files/last.txt\"\n",
    );
    common::write(&dir, "z-blocked", "not a directory\n");

    let output = run(
        &dir,
        &["recipe", "install", "mine"],
        closed_pipe(),
        Stdio::piped(),
    );

    assert_eq!(output.status.code(), Some(2), "{}", common::stderr(&output));
    assert!(
        common::stderr(&output).contains("z-blocked"),
        "{}",
        common::stderr(&output)
    );
    assert!(!common::stderr(&output).contains("Broken pipe"));
    assert_eq!(
        std::fs::read_to_string(dir.join("first.txt")).expect("first step completed"),
        "template\n"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("z-blocked")).expect("existing file"),
        "not a directory\n"
    );
    assert!(!dir.join("z-blocked/last.txt").exists());
}

#[test]
fn joined_closed_stdout_and_stderr_preserve_lint_results_and_timings() {
    let dir = common::project_in(
        "broken-pipe",
        "joined",
        Some("select = [\"file-size-guard\"]\n"),
    );
    common::write(&dir, "big.py", &common::lines(701));
    let (reader, writer) = io::pipe().expect("create shared pipe");
    drop(reader);
    let stderr = writer.try_clone().expect("duplicate writer");

    let output = run(
        &dir,
        &["lint", "big.py", "--no-astgrep", "--timings"],
        writer.into(),
        stderr.into(),
    );

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn lint_run_errors_survive_a_discarded_report_even_with_exit_zero() {
    let dir = common::project_in(
        "broken-pipe",
        "run-error",
        Some("select = [\"file-size-guard\"]\n"),
    );
    common::write(&dir, "big.py", &common::lines(701));
    let missing = dir.join("no-such-extension").to_string_lossy().into_owned();
    for format in ["text", "json", "sarif"] {
        for exit_zero in [false, true] {
            let mut args = vec![
                "lint",
                "big.py",
                "--no-astgrep",
                "--ext",
                &missing,
                "--output-format",
                format,
            ];
            if exit_zero {
                args.push("--exit-zero");
            }
            let normal = common::command(&dir, &args);
            assert_eq!(
                normal.status.code(),
                Some(2),
                "{args:?}: {}",
                common::stderr(&normal)
            );
            assert!(!normal.stdout.is_empty());
            match format {
                "text" => assert!(
                    common::stderr(&normal).contains("no-such-extension"),
                    "{}",
                    common::stderr(&normal)
                ),
                "json" => {
                    let document: serde_json::Value =
                        serde_json::from_slice(&normal.stdout).expect("JSON report");
                    assert!(!document["errors"]
                        .as_array()
                        .expect("run errors")
                        .is_empty());
                }
                "sarif" => {
                    let document: serde_json::Value =
                        serde_json::from_slice(&normal.stdout).expect("SARIF report");
                    assert_eq!(
                        document["runs"][0]["invocations"][0]["executionSuccessful"],
                        false
                    );
                }
                _ => unreachable!(),
            }

            let output = run(&dir, &args, closed_pipe(), Stdio::piped());

            assert_eq!(
                output.status.code(),
                Some(2),
                "{args:?}: {}",
                common::stderr(&output)
            );
            assert_eq!(output.stderr, normal.stderr, "{args:?}");
        }
    }
}
