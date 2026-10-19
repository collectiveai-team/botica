//! `botica rules new <family>/<slug> --from <judgment rule>`: a deterministic rule that starts from
//! the text of a judgment rule and covers it.

mod common;

use std::path::Path;

use common::judgment::{project, read, ROOT};
use common::{command, stderr};

fn new_from(dir: &Path, kind: &str, from: &str) -> std::process::Output {
    command(
        dir,
        &[
            "rules",
            "new",
            "quality/no-exec",
            "--kind",
            kind,
            "--language",
            "python",
            "--from",
            from,
            "--catalog",
            ROOT,
        ],
    )
}

#[test]
fn the_new_rule_takes_the_text_of_the_judgment_rule_and_is_added_to_its_covered_by() {
    let dir = project("rules-new-from", "copied", "");

    let output = new_from(&dir, "ast-grep", "acme/judge-it");

    assert!(output.status.success(), "{}", stderr(&output));
    let folder = format!("{ROOT}/quality/no-exec");
    assert_eq!(
        read(&dir, &format!("{folder}/README.md")),
        "## Directive\n\nJudge it well. More text.\n\n## Why\n\nw\n\n## Exceptions\n\nn\n\n\
         ## Do\n\ndo\n\n## Don't\n\ndont\n\n\
         ## Suppression\n\nTODO: the comment or the configuration that silences a finding.\n"
    );
    let toml = read(&dir, &format!("{folder}/rule.toml"));
    assert!(toml.contains("summary = \"s\"\n"), "{toml}");
    assert!(
        read(&dir, &format!("{ROOT}/quality/judge-it/rule.toml"))
            .contains("covered-by = [\"acme/no-eval\", \"acme/no-exec\"]\n"),
        "{}",
        read(&dir, &format!("{ROOT}/quality/judge-it/rule.toml"))
    );
    let listed = command(&dir, &["rules", "--all"]);
    assert!(
        listed.status.success(),
        "the catalog loads: {}",
        stderr(&listed)
    );
}

#[test]
fn from_names_a_judgment_rule_and_writes_a_deterministic_one() {
    let dir = project("rules-new-from", "refused", "");

    let not_judgment = new_from(&dir, "ast-grep", "acme/no-eval");
    assert_eq!(not_judgment.status.code(), Some(2));
    assert!(
        stderr(&not_judgment).contains("`acme/no-eval` is not a judgment rule"),
        "{}",
        stderr(&not_judgment)
    );

    let judgment = new_from(&dir, "judgment", "acme/judge-it");
    assert_eq!(judgment.status.code(), Some(2));
    assert!(
        stderr(&judgment).contains("--from writes a deterministic rule"),
        "{}",
        stderr(&judgment)
    );
    assert!(!dir.join(ROOT).join("quality/no-exec").exists());
}
