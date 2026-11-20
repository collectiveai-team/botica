//! `review-coverage-current` in `botica lint`: the run reads the roles of `[review]` and the files
//! their blocks cover from the working tree, and fails on a stale block.

mod common;

use common::{botica, project_in, stdout, write};

const SELECTED: &str = "select = [\"review-coverage-current\"]\npreview = true\n\n\
                        [review]\nroles = [\"tests\"]\n";
const CI: &str = "jobs:\n  test:\n    runs-on: ubuntu-latest\n";
const ROLE: &str =
    "---\nrole: tests\ncovered:\n  jobs:\n    - file: .github/workflows/ci.yml\n      \
                    id: test\n---\n\n";

fn block(ci: &str) -> String {
    format!(
        "<!-- botica:covered -->\n- job \"test\" in \".github/workflows/ci.yml\": blake3 {}\n\
         <!-- /botica:covered -->\n",
        blake3::hash(ci.as_bytes()).to_hex()
    )
}

#[test]
fn an_edit_of_a_covered_workflow_fails_the_run_until_the_block_is_refreshed() {
    let dir = project_in("review-coverage-current", "stale", Some(SELECTED));
    write(&dir, ".github/workflows/ci.yml", CI);
    write(
        &dir,
        ".github/review/tests/REVIEW.md",
        &format!("{ROLE}{}", block(CI)),
    );
    let current = botica(&dir, &[]);
    assert!(current.status.success(), "{}", stdout(&current));

    let edited = CI.replace("ubuntu", "macos");
    write(&dir, ".github/workflows/ci.yml", &edited);
    let stale = botica(&dir, &[]);
    let text = stdout(&stale);
    assert!(!stale.status.success(), "{text}");
    assert!(
        text.contains(
            ".github/review/tests/REVIEW.md:9:1: error [review-coverage-current] This coverage \
             block is stale"
        ),
        "{text}"
    );
    assert!(
        text.contains(&blake3::hash(edited.as_bytes()).to_hex().to_string()),
        "{text}"
    );

    write(
        &dir,
        ".github/review/tests/REVIEW.md",
        &format!("{ROLE}{}", block(&edited)),
    );
    assert!(botica(&dir, &[]).status.success());
}

#[cfg(unix)]
#[test]
fn review_dir_moves_the_roles_and_a_symbolic_link_is_refused() {
    let config = SELECTED.replace("[review]\n", "[review]\ndir = \"review\"\n");
    let dir = project_in("review-coverage-current", "dir", Some(&config));
    write(&dir, "shared/ci.yml", CI);
    write(
        &dir,
        "review/tests/REVIEW.md",
        &format!("{ROLE}{}", block(CI)),
    );
    std::fs::create_dir_all(dir.join(".github/workflows")).unwrap();
    std::os::unix::fs::symlink("../../shared/ci.yml", dir.join(".github/workflows/ci.yml"))
        .unwrap();

    let text = stdout(&botica(&dir, &[]));
    assert!(
        text.contains("review/tests/REVIEW.md:1:1: error [review-coverage-current]"),
        "{text}"
    );
    assert!(text.contains("is not a regular file"), "{text}");
}
