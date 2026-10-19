use super::*;

#[test]
fn a_notebook_inside_the_code_is_reported_whatever_the_ecosystem_and_one_beside_it_is_not() {
    let dir = project("notebooks");
    backend(&dir);
    write(&dir, "backend/amarc/analysis.ipynb", "{}\n");
    write(&dir, "backend/notebooks/explore.ipynb", "{}\n");
    write(&dir, "frontend/package.json", "{}\n");
    write(&dir, "frontend/src/chart.ipynb", "{}\n");
    write(&dir, "frontend/notebooks/explore.ipynb", "{}\n");
    write(&dir, "notebooks/root.ipynb", "{}\n");

    let output = command(&dir, &["lint", ".", "--no-astgrep"]);

    let text = stdout(&output);
    assert_eq!(
        reported(&text, "no-notebooks"),
        ["backend/amarc/analysis.ipynb", "frontend/src/chart.ipynb"],
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn a_file_that_only_mentions_ipynb_is_not_a_notebook() {
    let dir = project("notebook-lookalikes");
    backend(&dir);
    write(&dir, "backend/amarc/notes.ipynb.txt", "x\n");
    write(&dir, "backend/amarc/ipynb", "x\n");

    let text = lint(&dir);

    assert_eq!(reported(&text, "no-notebooks"), nothing(), "{text}");
}

#[test]
fn a_notebook_nobody_may_read_is_still_reported_by_its_name_and_is_no_failure() {
    use std::os::unix::fs::PermissionsExt;
    let dir = project("notebook-locked");
    backend(&dir);
    write(&dir, "backend/amarc/secret.ipynb", "{}\n");
    let path = dir.join("backend/amarc/secret.ipynb");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    if std::fs::File::open(&path).is_ok() {
        eprintln!("skipped: the file is readable by this user");
        return;
    }

    let output = command(
        &dir,
        &["lint", ".", "--no-astgrep", "--select", "no-notebooks"],
    );

    let text = stdout(&output);
    assert_eq!(
        reported(&text, "no-notebooks"),
        ["backend/amarc/secret.ipynb"],
        "{text}"
    );
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
}

#[test]
fn what_tests_and_documentation_carry_is_left_alone_as_for_data() {
    let dir = project("notebook-exempt");
    backend(&dir);
    for place in [
        "tests",
        "fixtures",
        "testdata",
        "examples",
        "docs",
        "snapshots",
        "api_tests",
        "e2e-tests",
    ] {
        write(&dir, &format!("backend/amarc/{place}/demo.ipynb"), "{}\n");
    }
    write(&dir, "backend/amarc/core/examples/demo.ipynb", "{}\n");
    write(&dir, "backend/amarc/analysis.ipynb", "{}\n");

    let text = lint(&dir);

    assert_eq!(
        reported(&text, "no-notebooks"),
        ["backend/amarc/analysis.ipynb"],
        "{text}"
    );
}
