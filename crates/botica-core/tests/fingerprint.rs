use botica_core::fingerprint::{FingerprintKey, ShortKey};
use botica_core::{Diagnostic, Severity};

const KEY: &str = "0123456789abcdef0123456789abcdef";

fn unkeyed() -> FingerprintKey {
    FingerprintKey::new(None).expect("no key is a valid key")
}

fn finding(path: &str, rule: &str, line: usize) -> Diagnostic {
    Diagnostic {
        path: path.to_owned(),
        rule: rule.to_owned(),
        severity: Severity::Warning,
        start: 0,
        end: 0,
        line,
        column: 1,
        end_line: line,
        end_column: 1,
        file_level: false,
        message: "a message".to_owned(),
        fingerprint: None,
        measure: None,
    }
}

// The expected values come from the reference implementation of the `blake3` package of Python:
// the key is `blake3(value, derive_key_context="botica history fingerprint v1")`, and each field is
// hashed after its length as a 64-bit little-endian integer.

#[test]
fn a_finding_in_a_line_hashes_the_line_and_its_nearest_non_blank_neighbors() {
    let text = "def main():\n\n    print(x)\n   \n    return 1\n";

    let fingerprint = unkeyed().of_line("no-print", "src/a.py", text, 3);

    assert_eq!(fingerprint, "fc6c37b1e6a10db3");
}

#[test]
fn the_key_of_botica_history_key_changes_every_fingerprint() {
    let text = "def main():\n    print(x)\n    return 1\n";
    let key = FingerprintKey::new(Some(KEY)).expect("a key of 32 bytes");

    assert_eq!(
        key.of_line("no-print", "src/a.py", text, 2),
        "2f92438af2df38b3"
    );
    assert_eq!(
        FingerprintKey::new(Some("")).map(|key| key.of_file("r", "p")),
        Ok(unkeyed().of_file("r", "p"))
    );
}

#[test]
fn a_finding_about_a_whole_file_has_three_empty_lines() {
    assert_eq!(
        unkeyed().of_file("file-size-guard", "big.py"),
        "3859838336ed4b85"
    );
}

#[test]
fn a_finding_with_no_line_text_hashes_its_normalized_message() {
    assert_eq!(
        unkeyed().of_message("conventional-commits", "", "  the subject\thas  no type\n"),
        "7d68e2a43cb6636a"
    );
}

#[test]
fn a_key_shorter_than_32_bytes_is_refused() {
    assert_eq!(
        FingerprintKey::new(Some("short")).err(),
        Some(ShortKey { len: 5 })
    );
}

#[test]
fn the_fingerprint_does_not_change_when_the_finding_moves_down() {
    let before = "import os\n\ndef main():\n    print(x)\n    return 1\n";
    let after = "import os\nimport sys\n\n\ndef main():\n    print(x)\n    return 1\n";
    let mut found = [finding("src/a.py", "no-print", 4)];
    let mut moved = [finding("src/a.py", "no-print", 6)];

    unkeyed().stamp(&mut found, str::to_owned, |_| Some(before.to_owned()));
    unkeyed().stamp(&mut moved, str::to_owned, |_| Some(after.to_owned()));

    assert!(found[0].fingerprint.is_some());
    assert_eq!(found[0].fingerprint, moved[0].fingerprint);
}

#[test]
fn whitespace_inside_and_around_the_lines_does_not_change_it() {
    let plain = "a = 1\nprint(x)\nb = 2\n";
    let spaced = "  a  =  1\t\n\tprint(x)   \nb =  2\n";

    assert_eq!(
        unkeyed().of_line("no-print", "a.py", plain, 2),
        unkeyed().of_line("no-print", "a.py", spaced, 2)
    );
}

#[test]
fn two_equal_lines_of_a_file_are_told_apart_by_their_neighbors() {
    let text = "def a():\n    print(x)\n\ndef b():\n    print(x)\n";

    assert_ne!(
        unkeyed().of_line("no-print", "a.py", text, 2),
        unkeyed().of_line("no-print", "a.py", text, 5)
    );
}

#[test]
fn an_edit_of_a_neighbor_the_rule_or_the_path_starts_a_new_fingerprint() {
    let text = "a = 1\nprint(x)\nb = 2\n";
    let first = unkeyed().of_line("no-print", "a.py", text, 2);

    assert_ne!(
        first,
        unkeyed().of_line("no-print", "a.py", "a = 2\nprint(x)\nb = 2\n", 2)
    );
    assert_ne!(first, unkeyed().of_line("other-rule", "a.py", text, 2));
    assert_ne!(first, unkeyed().of_line("no-print", "b.py", text, 2));
}

#[test]
fn the_length_of_each_field_keeps_the_fields_apart() {
    assert_ne!(unkeyed().of_file("ab", "c"), unkeyed().of_file("a", "bc"));
}

#[test]
fn stamp_hashes_the_path_from_the_root_of_the_project_and_reads_each_file_once() {
    let mut reads = 0;
    let mut findings = [
        finding("a.py", "no-print", 2),
        finding("a.py", "no-print", 3),
        Diagnostic::file_level("a.py", "file-size-guard", Severity::Error, "big".to_owned()),
        finding("", "a-change-rule", 0),
    ];
    let text = "x = 1\nprint(x)\nprint(y)\n";

    unkeyed().stamp(
        &mut findings,
        |path| format!("pkg/{path}"),
        |_| {
            reads += 1;
            Some(text.to_owned())
        },
    );

    assert_eq!(reads, 1);
    let fingerprints: Vec<_> = findings
        .iter()
        .map(|finding| finding.fingerprint.clone().expect("each finding has one"))
        .collect();
    assert_eq!(
        fingerprints,
        [
            unkeyed().of_line("no-print", "pkg/a.py", text, 2),
            unkeyed().of_line("no-print", "pkg/a.py", text, 3),
            unkeyed().of_file("file-size-guard", "pkg/a.py"),
            unkeyed().of_message("a-change-rule", "", "a message"),
        ]
    );
}

#[test]
fn stamp_falls_back_to_the_whole_file_when_the_text_cannot_be_read() {
    let mut findings = [finding("gone.py", "no-print", 2)];

    unkeyed().stamp(&mut findings, str::to_owned, |_| None);

    assert_eq!(
        findings[0].fingerprint.as_deref(),
        Some(unkeyed().of_file("no-print", "gone.py").as_str())
    );
}
