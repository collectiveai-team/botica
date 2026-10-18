//! Runs the command of a step of `botica recipe install` and says why it failed.

use std::process::Command;

/// Runs `command` and says why it failed, when it did: `label` followed by how it ended and the
/// last `cause:` line of its stderr, or its last line when there is none, leaving out the `hint:`
/// lines, or the reason in its stdout when stderr has nothing else.
pub fn failure(command: &mut Command, label: &str) -> Option<String> {
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            let program = command.get_program().to_string_lossy().into_owned();
            return Some(format!("cannot run `{program}`: {error}"));
        }
    };
    if output.status.success() {
        return None;
    }
    let how = match output.status.code() {
        Some(code) => format!("exited with code {code}"),
        None => "was stopped by a signal".to_owned(),
    };
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("hint:"))
        .collect();
    let last = lines
        .iter()
        .rev()
        .find(|line| line.starts_with("cause:"))
        .or(lines.last())
        .map(|line| (*line).to_owned())
        .or_else(|| stdout_reason(&String::from_utf8_lossy(&output.stdout)));
    Some(match last {
        Some(line) => format!("{label} {how}: {line}"),
        None => format!("{label} {how}"),
    })
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

/// Why the CLI of skills failed, from its stdout, where it writes its errors and leaves stderr
/// empty. Each error starts with `■`: the reason is the first one that is not `Canceled`, followed
/// by the last `fatal:` or `error:` line of git below it.
fn stdout_reason(stdout: &str) -> Option<String> {
    let clean = without_escapes(stdout);
    let headline = clean
        .lines()
        .filter_map(|line| line.split_once('■'))
        .map(|(_, text)| text.trim())
        .find(|text| !text.is_empty() && *text != "Canceled");
    let detail = clean
        .lines()
        .map(|line| line.trim_start_matches(['│', ' ']).trim_end())
        .rfind(|line| line.starts_with("fatal:") || line.starts_with("error:"));
    match (headline, detail) {
        (Some(headline), Some(detail)) => Some(format!("{headline}: {detail}")),
        (Some(text), None) | (None, Some(text)) => Some(text.to_owned()),
        (None, None) => None,
    }
}
