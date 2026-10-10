mod common;

use common::{command, project_in, stderr, write};

#[test]
fn the_measurement_options_are_gone() {
    let dir = project_in("removed_options", "gone", None);
    write(&dir, "a.py", "x = 1\n");
    for switch in [
        &["--schedule", "timed"][..],
        &["--schedule", "largest-first"],
        &["--separate-walk"],
        &["--describe-prefilter"],
    ] {
        let mut args = vec!["lint", "."];
        args.extend_from_slice(switch);
        let output = command(&dir, &args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{switch:?} is still accepted"
        );
        assert!(
            stderr(&output).contains("unexpected argument"),
            "{switch:?}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn timings_show_rules_and_the_total_but_no_phases() {
    let dir = project_in("removed_options", "timings", None);
    write(&dir, "a.py", "x = 1\n");
    let text = stderr(&command(&dir, &["lint", ".", "--timings"]));

    assert!(text.contains("[timing] rule file-size-guard"), "{text}");
    assert!(text.contains("[timing] total"), "{text}");
    for phase in ["compile rules", "walk", "read", "cpu:", "native+ast-grep"] {
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with(&format!("[timing] {phase}")))
            .collect();
        assert!(
            lines.is_empty(),
            "a `{phase}` phase line is left: {lines:?}"
        );
    }
}
