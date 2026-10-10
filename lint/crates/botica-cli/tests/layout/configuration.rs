use super::*;

fn code_dirs_of(dir: &Path) -> String {
    let output = command(dir, &["explain", "no-notebooks"]);
    let text = stdout(&output);
    text.lines()
        .find(|line| line.starts_with("code-dirs:"))
        .unwrap_or_else(|| panic!("explain has no `code-dirs:` line:\n{text}"))
        .to_owned()
}

#[test]
fn explain_lists_what_it_detected_and_says_where_it_comes_from() {
    let dir = project("explain-detected");
    backend(&dir);
    write(&dir, "frontend/package.json", "{}\n");
    write(&dir, "frontend/src/index.ts", "export {};\n");

    assert_eq!(
        code_dirs_of(&dir),
        "code-dirs: python: backend/amarc, js: frontend/src (detected)"
    );
}

#[test]
fn explain_says_the_rules_do_nothing_when_there_is_no_code() {
    let dir = project("explain-none");
    write(&dir, "data/rows.csv", "a,b\n");

    let line = code_dirs_of(&dir);

    assert!(line.starts_with("code-dirs: none (detected)"), "{line}");
    assert!(line.contains("do nothing"), "{line}");
}

#[test]
fn the_configuration_replaces_the_detection() {
    let dir = project("configured");
    write(&dir, "botica.toml", "[code-dirs]\npython = [\"lib\"]\n");
    write(&dir, "lib/tests/test_a.py", "x = 1\n");
    backend(&dir);
    write(&dir, "backend/amarc/test_b.py", "x = 1\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-tests-in-package"),
        ["lib/tests/test_a.py"],
        "{text}"
    );
    assert_eq!(code_dirs_of(&dir), "code-dirs: python: lib (configuration)");
}

#[test]
fn an_empty_table_means_there_is_no_code() {
    let dir = project("none-configured");
    write(&dir, "botica.toml", "[code-dirs]\n");
    backend(&dir);
    write(&dir, "backend/amarc/test_b.py", "x = 1\n");

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
}

#[test]
fn an_ecosystem_that_is_not_known_is_a_configuration_error() {
    let dir = project("bad-ecosystem");
    write(&dir, "botica.toml", "[code-dirs]\ncobol = [\"src\"]\n");

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("code-dirs.cobol"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_rule_is_switched_off_apart_from_the_others() {
    let dir = project("apart");
    backend(&dir);
    write(&dir, "backend/amarc/test_a.py", "x = 1\n");
    write(&dir, "backend/amarc/a.ipynb", "{}\n");
    write(&dir, "backend/amarc/a.csv", "x\n");
    let lint_with = |args: &[&str]| {
        let mut all = vec!["lint", ".", "--no-astgrep"];
        all.extend_from_slice(args);
        stdout(&command(&dir, &all))
    };

    let all = lint_with(&[]);
    assert_eq!(reported(&all, "no-tests-in-package").len(), 1, "{all}");
    assert_eq!(reported(&all, "no-notebooks").len(), 1, "{all}");
    assert_eq!(reported(&all, "no-artifacts-in-package").len(), 1, "{all}");

    let without = lint_with(&["--ignore", "no-notebooks"]);
    assert_eq!(reported(&without, "no-notebooks"), nothing());
    assert_eq!(reported(&without, "no-tests-in-package").len(), 1);
    assert_eq!(reported(&without, "no-artifacts-in-package").len(), 1);
}
