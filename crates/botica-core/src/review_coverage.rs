//! `review-coverage-current`: the `<!-- botica:covered -->` block of each review role matches the
//! hooks and jobs it declares, as `botica review check` computes them, read from the working tree.

use anyhow::Result;

use crate::diagnostic::{Diagnostic, Severity};
use crate::document::Document;
use crate::review::coverage::{check_references, current_block, render, OPEN};
use crate::review::roles::{Covered, RoleDefinition};

pub const REVIEW_COVERAGE_CURRENT: &str = "review-coverage-current";

/// Reads a source file from the root of the project: its bytes, or `None` when it is missing.
pub type Read<'a> = &'a dyn Fn(&str) -> Result<Option<Vec<u8>>>;

/// A `REVIEW.md` of a role, from the root of the project, and its text. The role is the name of
/// its folder.
pub struct Role<'a> {
    pub path: &'a str,
    pub text: &'a str,
}

/// The findings for `roles`, one at most for each.
pub fn check(roles: &[Role<'_>], read: Read<'_>, severity: Severity) -> Vec<Diagnostic> {
    roles
        .iter()
        .filter_map(|role| {
            let (on_block, message) = problem(role, read)?;
            Some(diagnostic(role, on_block, severity, message))
        })
        .collect()
}

/// What is wrong with the coverage of `role`, and whether the finding sits on its opening marker.
fn problem(role: &Role<'_>, read: Read<'_>) -> Option<(bool, String)> {
    let name = role.path.rsplit('/').nth(1).unwrap_or_default();
    let definition = match RoleDefinition::parse(name, role.text) {
        Ok(definition) => definition,
        Err(error) => {
            return Some((
                false,
                format!("botica review check cannot read this role: {error:#}"),
            ))
        }
    };
    let Some(covered) = definition.covered else {
        return Some((
            false,
            "This role declares no `covered` in its front matter, so botica review check reports \
             its coverage as unverified: add `covered: {}`, or list the hooks and jobs it relies \
             on"
            .to_owned(),
        ));
    };
    let expected = match expected_blocks(&covered, read) {
        Ok(expected) => expected,
        Err(error) => {
            return Some((
                false,
                format!("botica review check cannot verify the coverage of this role: {error:#}"),
            ))
        }
    };
    let actual = match current_block(role.text) {
        Ok(Some(actual)) => actual,
        Ok(None) => {
            return Some((
                false,
                format!(
                    "This role has no coverage block: add the block that botica review check \
                     expects\n{}",
                    expected[0]
                ),
            ))
        }
        Err(error) => {
            return Some((
                false,
                format!("The coverage block of this role is malformed: {error:#}"),
            ))
        }
    };
    // With rules declared, the rule lines bind the version of botica and its configuration, which
    // lint does not compute, so only the hook and job lines are compared.
    let current = |wanted: &String| {
        if covered.rules.is_empty() {
            &actual == wanted
        } else {
            check_lines(&actual) == check_lines(wanted)
        }
    };
    if expected.iter().any(current) {
        return None;
    }
    let shown = if covered.rules.is_empty() {
        expected[0].clone()
    } else {
        check_lines(&expected[0]).join("\n")
    };
    Some((
        true,
        format!(
            "This coverage block is stale: a file it covers changed, or its declarations did. \
             Replace its hook and job lines with these, which botica review check expects\n{shown}"
        ),
    ))
}

/// The block from the bytes of the sources, then from the same bytes with CRLF read as LF: git
/// stores LF, so a checkout with CRLF line ends still matches the block that review check writes.
fn expected_blocks(covered: &Covered, read: Read<'_>) -> Result<[String; 2]> {
    let as_lf = |path: &str| -> Result<Option<Vec<u8>>> {
        Ok(read(path)?.map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .replace("\r\n", "\n")
                .into_bytes()
        }))
    };
    Ok([block(covered, read)?, block(covered, &as_lf)?])
}

fn block(covered: &Covered, read: Read<'_>) -> Result<String> {
    let mut references = check_references(covered, read)?;
    render(&mut references)
}

fn check_lines(block: &str) -> Vec<&str> {
    block
        .lines()
        .filter(|line| line.starts_with("- hook ") || line.starts_with("- job "))
        .collect()
}

fn diagnostic(role: &Role<'_>, on_block: bool, severity: Severity, message: String) -> Diagnostic {
    let start = role.text.find(OPEN).filter(|_| on_block);
    let Some(start) = start else {
        return Diagnostic::file_level(role.path, REVIEW_COVERAGE_CURRENT, severity, message);
    };
    let end = start + OPEN.len();
    let document = Document::new(role.path, role.text);
    let (line, column) = document.position(start);
    let (end_line, end_column) = document.position(end);
    Diagnostic {
        path: role.path.to_owned(),
        rule: REVIEW_COVERAGE_CURRENT.to_owned(),
        severity,
        start,
        end,
        line,
        column,
        end_line,
        end_column,
        file_level: false,
        fingerprint: None,
        measure: None,
        message,
    }
}
