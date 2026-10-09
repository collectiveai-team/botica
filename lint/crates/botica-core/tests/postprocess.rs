use botica_core::postprocess::{apply, Verdict};
use botica_core::{Diagnostic, Severity};

fn found(path: &str, rule: &str, start: usize) -> Diagnostic {
    Diagnostic {
        path: path.to_owned(),
        rule: rule.to_owned(),
        severity: Severity::Warning,
        start,
        end: start + 1,
        line: 1,
        column: start + 1,
        file_level: false,
        message: String::new(),
    }
}

fn keep_all(_path: &str, _rule: &str) -> Verdict {
    Verdict::Keep
}

fn order(diagnostics: &[Diagnostic]) -> Vec<(&str, usize, &str)> {
    diagnostics
        .iter()
        .map(|d| (d.path.as_str(), d.start, d.rule.as_str()))
        .collect()
}

#[test]
fn diagnostics_come_out_ordered_by_path_then_start_then_rule() {
    let engine_and_extension = vec![
        found("b.py", "no-utils", 0),
        found("a.py", "z-rule", 9),
        found("a.py", "b-rule", 4),
        found("a.py", "a-rule", 4),
        found("a.py", "m-rule", 0),
    ];

    let out = apply(engine_and_extension, keep_all);

    assert_eq!(
        order(&out),
        [
            ("a.py", 0, "m-rule"),
            ("a.py", 4, "a-rule"),
            ("a.py", 4, "b-rule"),
            ("a.py", 9, "z-rule"),
            ("b.py", 0, "no-utils"),
        ]
    );
}

#[test]
fn the_severity_the_configuration_gives_replaces_the_one_the_rule_reported() {
    let diagnostics = vec![found("a.py", "no-utils", 0), found("a.py", "other", 1)];

    let out = apply(diagnostics, |_, rule| match rule {
        "no-utils" => Verdict::Report(Severity::Error),
        _ => Verdict::Keep,
    });

    assert_eq!(out[0].severity, Severity::Error);
    assert_eq!(out[1].severity, Severity::Warning, "Keep leaves it alone");
}

#[test]
fn a_rule_that_does_not_apply_at_a_path_loses_its_diagnostics_there_only() {
    let diagnostics = vec![
        found("src/a.py", "unused-suppression", 0),
        found("tests/a.py", "unused-suppression", 0),
        found("tests/a.py", "other", 5),
    ];

    let out = apply(diagnostics, |path, rule| {
        if path.starts_with("tests/") && rule == "unused-suppression" {
            Verdict::Drop
        } else {
            Verdict::Keep
        }
    });

    assert_eq!(
        order(&out),
        [
            ("src/a.py", 0, "unused-suppression"),
            ("tests/a.py", 5, "other")
        ]
    );
}
