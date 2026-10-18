//! Runs the command of a step of `botica recipe install` and says why it failed.

use std::process::Command;

use crate::recipes_install_json::failed_reason;

/// What a command did: why it failed, when it did, and what it wrote to stdout.
pub struct Ran {
    pub failure: Option<String>,
    pub stdout: String,
}

/// Runs `command` and says why it failed, when it did: `label` followed by how it ended and the
/// reason that the JSON of its stdout gives, or else the last `cause:` line of its stderr, or its
/// last line when there is none, leaving out the `hint:` lines.
pub fn run(command: &mut Command, label: &str) -> Ran {
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            let program = command.get_program().to_string_lossy().into_owned();
            return Ran {
                failure: Some(format!("cannot run `{program}`: {error}")),
                stdout: String::new(),
            };
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if output.status.success() {
        return Ran {
            failure: None,
            stdout,
        };
    }
    let how = match output.status.code() {
        Some(code) => format!("exited with code {code}"),
        None => "was stopped by a signal".to_owned(),
    };
    let stderr = without_escapes(&String::from_utf8_lossy(&output.stderr));
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("hint:"))
        .collect();
    let last = failed_reason(&stdout).or_else(|| {
        lines
            .iter()
            .rev()
            .find(|line| line.starts_with("cause:"))
            .or(lines.last())
            .map(|line| (*line).to_owned())
    });
    let failure = match last {
        Some(line) => format!("{label} {how}: {line}"),
        None => format!("{label} {how}"),
    };
    Ran {
        failure: Some(failure),
        stdout,
    }
}

/// Runs `command` and says why it failed, when it did.
pub fn failure(command: &mut Command, label: &str) -> Option<String> {
    run(command, label).failure
}

/// The text of `text` without ANSI escape sequences (`ESC [ … <letter>`).
fn without_escapes(text: &str) -> String {
    let mut clean = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            clean.push(c);
        } else if chars.as_str().starts_with('[') {
            chars.next();
            for end in chars.by_ref() {
                if ('@'..='~').contains(&end) {
                    break;
                }
            }
        }
    }
    clean
}
