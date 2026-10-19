//! `botica refs show` and `botica refs check` read the files of external references with `git`.
//!
//! No test reaches the network: `https://<repo>.git` is a repository on disk (`common/git_remote`).

mod common;

use std::path::Path;

use common::external::{world, REPO};
use common::git_remote::{refs_at, Origin};
use common::project_in;
use common::refs::{add_topic, failed, ok};

#[test]
fn show_prints_the_lines_of_a_file_of_another_repository_at_its_commit() {
    let world = world("show");
    world.topic(REPO, "lines = \"2-3\"\n");

    let text = ok(&world.run(REPO, &["show", "demo"]));

    let sha = &world.first;
    let expected = format!(
        "## {REPO}@{sha} src/a.py:2-3\n\nthe setup\n\nlicense: MIT\n\
         https://github.com/acme/app/blob/{sha}/src/a.py#L2-L3\n\n```\ntwo\nthree\n```\n"
    );
    assert!(text.contains(&expected), "{text}");
    assert!(
        !text.contains("changed"),
        "the commit fixes the lines:\n{text}"
    );
    assert!(
        !text.contains("one"),
        "only the lines of the reference:\n{text}"
    );
}

#[test]
fn the_permalink_has_a_single_line_or_none_and_only_for_github() {
    let world = world("permalink");
    world.topic(REPO, "lines = \"3\"\n");
    let text = ok(&world.run(REPO, &["show", "demo"]));
    assert!(text.contains("/src/a.py#L3\n"), "{text}");

    world.topic(REPO, "");
    let text = ok(&world.run(REPO, &["show", "demo"]));
    assert!(text.contains("/src/a.py\n"), "{text}");
    assert!(!text.contains("#L"), "{text}");

    world.topic("gitlab.example.com/acme/app", "");
    let text = ok(&world.run("gitlab.example.com/acme/app", &["show", "demo"]));
    assert!(text.contains("license: MIT\n"), "{text}");
    assert!(!text.contains("/blob/"), "no permalink off GitHub:\n{text}");
}

#[test]
fn show_names_the_repository_and_the_commit_when_it_cannot_fetch() {
    let world = world("unreachable");
    world.topic("example.invalid/acme/app", "");

    let text = failed(&world.run_offline(&["show", "demo"]), 2);

    let sha = &world.first;
    assert!(
        text.contains(&format!("cannot fetch example.invalid/acme/app@{sha}: ")),
        "{text}"
    );
}

#[test]
fn show_says_so_when_the_commit_is_not_in_the_repository() {
    let world = world("no-commit");
    world.topic_at(REPO, &"0".repeat(40), "");

    let text = failed(&world.run(REPO, &["show", "demo"]), 2);

    assert!(
        text.contains(&format!("cannot fetch {REPO}@{}: ", "0".repeat(40))),
        "{text}"
    );
}

#[test]
fn show_says_so_when_the_path_is_not_a_file_of_the_commit() {
    let world = world("no-path");
    world.topic(REPO, "");
    world.point_at("src/none.py");

    let text = failed(&world.run(REPO, &["show", "demo"]), 2);

    assert!(
        text.contains(&format!(
            "`src/none.py` is not a file in {REPO}@{}",
            world.first
        )),
        "{text}"
    );
}

#[test]
fn a_directory_is_not_a_file() {
    let world = world("a-directory");
    world.topic(REPO, "");
    world.point_at("src");

    let text = failed(&world.run(REPO, &["show", "demo"]), 2);

    assert!(text.contains("`src` is not a file in"), "{text}");
}

#[test]
fn a_file_that_is_not_text_is_refused() {
    let origin = Origin::new("binary");
    let sha = origin.commit(&[("src/a.py", &[0xff, 0xfe, 0x00])]);
    let dir = project_in("refs-external", "binary", None);
    add_topic(
        &dir,
        "demo",
        &format!(
            "title = \"E\"\n\n[[references]]\nrepo = \"{REPO}\"\nrev = \"{sha}\"\nlicense = \"MIT\"\n\
             path = \"src/a.py\"\nnote = \"n\"\n"
        ),
    );
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR")).join("refs-cache/binary");
    let _ = std::fs::remove_dir_all(&cache);

    let output = refs_at(
        &dir,
        &["show", "demo", "--cache-dir", &cache.display().to_string()],
        REPO,
        &origin,
    );

    let text = failed(&output, 2);
    assert!(text.contains("`src/a.py` is not UTF-8 text"), "{text}");
}

#[test]
fn a_path_that_leaves_the_repository_is_refused() {
    let world = world("outside");
    world.topic(REPO, "");
    world.point_at("../a.py");

    let text = failed(&world.run(REPO, &["show", "demo"]), 2);

    assert!(
        text.contains("`../a.py` must be a path inside the repository"),
        "{text}"
    );
}

#[test]
fn check_resolves_the_lines_and_the_anchor_in_the_file_of_the_commit() {
    let world = world("check-ok");
    world.topic(REPO, "lines = \"2-3\"\nanchor = \"three\"\n");

    let text = ok(&world.run(REPO, &["check"]));

    assert_eq!(text, "1 topic and 1 reference resolve\n");
}

#[test]
fn check_fails_when_the_lines_are_past_the_end_of_the_file_of_the_commit() {
    let world = world("check-lines");
    world.topic(REPO, "lines = \"2-9\"\n");

    let text = failed(&world.run(REPO, &["check"]), 1);

    assert!(text.contains("references/demo/topic.toml"), "{text}");
    assert!(text.contains("2-9"), "{text}");
    assert!(text.contains("3 lines"), "{text}");
}

#[test]
fn check_fails_when_the_anchor_is_not_in_the_lines() {
    let world = world("check-anchor");
    world.topic(REPO, "lines = \"1-2\"\nanchor = \"three\"\n");

    let text = failed(&world.run(REPO, &["check"]), 1);

    assert!(
        text.contains("the anchor `three` is not in lines 1-2"),
        "{text}"
    );
}

#[test]
fn check_reports_each_external_reference_that_fails_and_goes_on() {
    let world = world("check-many");
    world.topic(
        REPO,
        "\n[[references]]\nrepo = \"example.invalid/acme/app\"\nrev = \"0000000000000000000000000000000000000000\"\n\
         license = \"MIT\"\npath = \"a.py\"\nnote = \"n\"\n\n[[references]]\npath = \"gone.py\"\nnote = \"n\"\n",
    );

    let text = failed(&world.run(REPO, &["check"]), 1);

    assert!(
        text.contains("cannot fetch example.invalid/acme/app@000"),
        "{text}"
    );
    assert!(text.contains("gone.py"), "{text}");
    assert!(text.contains("2 problems"), "{text}");
}
