use super::{common, Path};

pub(super) fn assert_readable_history(dir: &Path, expected: usize) {
    let output = common::command(dir, &["history", "report"]);
    assert!(output.status.success(), "{}", common::stderr(&output));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains(&format!("{expected} runs of the local history")),
        "{text}"
    );
    assert!(!text.contains("lines failed their checks"), "{text}");
    assert!(
        !text.contains("runs of a mode or origin that this report does not know"),
        "{text}"
    );
}
