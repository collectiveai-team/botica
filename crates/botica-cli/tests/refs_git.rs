//! How `botica refs` runs `git`: its command line, its deadline and what it does when git fails.

mod common;

use std::time::{Duration, Instant};

use common::external::{world, REPO};
use common::git_remote::{refs_run, Setup};
use common::refs::{failed, ok};

#[test]
fn without_git_the_command_says_that_it_needs_it() {
    let world = world("no-git");
    world.topic(REPO, "");
    let empty = world.cache.join("empty-path");
    std::fs::create_dir_all(&empty).expect("create the folder");

    let cache = world.cache_arg();
    let output = refs_run(
        &world.dir,
        &["show", "demo", "--cache-dir", &cache],
        &Setup {
            env: vec![("PATH", empty.display().to_string())],
            ..Setup::default()
        },
    );

    let text = failed(&output, 2);
    assert!(
        text.contains("git is needed to read external references"),
        "{text}"
    );
}

#[cfg(unix)]
#[test]
fn a_git_that_does_not_answer_is_killed_at_the_deadline() {
    let world = world("deadline");
    world.topic(REPO, "");
    let path = world.fake_git("exec sleep 30");

    let cache = world.cache_arg();
    let started = Instant::now();
    let output = refs_run(
        &world.dir,
        &["show", "demo", "--cache-dir", &cache],
        &Setup {
            env: vec![("PATH", path), ("BOTICA_REFS_TIMEOUT", "1".to_owned())],
            ..Setup::default()
        },
    );

    let text = failed(&output, 2);
    assert!(
        text.contains("git did not answer within 1s and was killed"),
        "{text}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "{:?}",
        started.elapsed()
    );
}

#[cfg(unix)]
#[test]
fn git_is_told_never_to_ask_for_a_password() {
    let world = world("no-prompt");
    world.topic(REPO, "");
    let path = world.fake_git("echo \"prompt=$GIT_TERMINAL_PROMPT\" >&2\nexit 1");

    let cache = world.cache_arg();
    let output = refs_run(
        &world.dir,
        &["show", "demo", "--cache-dir", &cache],
        &Setup {
            env: vec![("PATH", path)],
            ..Setup::default()
        },
    );

    let text = failed(&output, 2);
    assert!(
        text.contains(&format!("cannot fetch {REPO}@{}: prompt=0", world.first)),
        "{text}"
    );
}

#[cfg(unix)]
#[test]
fn a_file_that_git_cannot_show_after_the_fetch_is_named_with_the_reason() {
    let world = world("unreadable");
    world.topic(REPO, "");
    let path = world.fake_git(
        "case \"$*\" in\n  *ls-tree*) echo '100644 blob abc' ;;\n  *show*) echo boom >&2; exit 1 ;;\n  *) exit 0 ;;\nesac",
    );

    let cache = world.cache_arg();
    let output = refs_run(
        &world.dir,
        &["show", "demo", "--cache-dir", &cache],
        &Setup {
            env: vec![("PATH", path)],
            ..Setup::default()
        },
    );

    let text = failed(&output, 2);
    let sha = &world.first;
    assert!(
        text.contains(&format!("cannot read `src/a.py` of {REPO}@{sha}: boom")),
        "{text}"
    );
}

#[cfg(unix)]
#[test]
fn the_fetch_takes_one_commit_and_no_file_contents() {
    let world = world("records-args");
    world.topic(REPO, "");
    let log = world.cache.join("git.log");
    let path = world.fake_git(&format!(
        "echo \"$*\" >> {}\ncase \"$*\" in *' fetch '*) exit 1 ;; esac\nexit 0",
        log.display()
    ));

    let cache = world.cache_arg();
    let output = refs_run(
        &world.dir,
        &["show", "demo", "--cache-dir", &cache],
        &Setup {
            env: vec![("PATH", path)],
            ..Setup::default()
        },
    );

    failed(&output, 2);
    let calls = std::fs::read_to_string(&log).expect("the log of the calls");
    let fetch = format!(
        "--depth 1 --filter=blob:none -- https://{REPO}.git {}\n",
        world.first
    );
    assert!(calls.contains(&fetch), "{calls}");
}

#[test]
fn a_timeout_that_is_not_a_positive_number_of_seconds_is_ignored() {
    for (index, value) in ["0", "soon", "-3", ""].into_iter().enumerate() {
        let world = world(&format!("timeout-{index}"));
        world.topic(REPO, "");
        let cache = world.cache_arg();

        let output = refs_run(
            &world.dir,
            &["show", "demo", "--cache-dir", &cache],
            &Setup {
                maps: vec![(REPO, &world.origin)],
                env: vec![("BOTICA_REFS_TIMEOUT", value.to_owned())],
                ..Setup::default()
            },
        );

        let text = ok(&output);
        assert!(text.contains("two"), "{value:?}: {text}");
    }
}
