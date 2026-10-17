//! The files of external references are kept by commit, in a cache that `--cache-dir` or the
//! environment places.

mod common;

use common::external::{world, FILE, REPO};
use common::git_remote::{refs_run, Setup};
use common::refs::{failed, ok};

#[test]
fn a_file_is_kept_by_commit_and_read_again_without_the_repository() {
    let world = world("cache");
    world.topic("example.invalid/acme/app", "lines = \"2-3\"\n");

    let first = ok(&world.run("example.invalid/acme/app", &["show", "demo"]));

    let kept = std::fs::read(world.cached("example.invalid/acme/app", "src/a.py"))
        .expect("the whole file is in the cache");
    assert_eq!(kept, FILE);
    let again = ok(&world.run_offline(&["show", "demo"]));
    assert_eq!(first, again);
}

#[test]
fn nothing_is_left_in_the_cache_but_the_files() {
    let world = world("cache-clean");
    world.topic("example.invalid/acme/app", "");
    ok(&world.run("example.invalid/acme/app", &["show", "demo"]));
    world.topic_at("example.invalid/acme/app", &"0".repeat(40), "");
    let _ = failed(&world.run("example.invalid/acme/app", &["show", "demo"]), 2);

    let mut names: Vec<String> = std::fs::read_dir(&world.cache)
        .expect("the cache")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();

    assert_eq!(names, ["example.invalid"], "no scratch directory stays");
}

#[test]
fn check_without_the_network_passes_for_what_the_cache_holds() {
    let world = world("check-cached");
    world.topic("example.invalid/acme/app", "lines = \"1\"\n");
    ok(&world.run("example.invalid/acme/app", &["check"]));

    let text = ok(&world.run_offline(&["check"]));

    assert_eq!(text, "1 topic and 1 reference resolve\n");
}

#[test]
fn check_without_the_network_fails_for_what_the_cache_does_not_hold() {
    let world = world("check-cold");
    world.topic("example.invalid/acme/app", "");

    let text = failed(&world.run_offline(&["check"]), 1);

    assert!(
        text.contains("cannot fetch example.invalid/acme/app@"),
        "{text}"
    );
}

#[test]
fn the_cache_is_under_xdg_cache_home_when_it_is_set() {
    let world = world("xdg");
    world.topic(REPO, "");
    let xdg = world.cache.join("xdg");

    let output = refs_run(
        &world.dir,
        &["show", "demo"],
        &Setup {
            maps: vec![(REPO, &world.origin)],
            env: vec![("XDG_CACHE_HOME", xdg.display().to_string())],
            removed: vec!["LOCALAPPDATA"],
        },
    );

    ok(&output);
    let file = xdg
        .join("botica/refs")
        .join(REPO)
        .join(&world.first)
        .join("src/a.py");
    assert!(file.is_file(), "{}", file.display());
}

#[test]
fn the_cache_is_under_home_when_xdg_cache_home_is_not_set() {
    let world = world("home");
    world.topic(REPO, "");
    let home = world.cache.join("home");

    let output = refs_run(
        &world.dir,
        &["show", "demo"],
        &Setup {
            maps: vec![(REPO, &world.origin)],
            env: vec![("HOME", home.display().to_string())],
            removed: vec!["XDG_CACHE_HOME", "LOCALAPPDATA"],
        },
    );

    ok(&output);
    let file = home
        .join(".cache/botica/refs")
        .join(REPO)
        .join(&world.first)
        .join("src/a.py");
    assert!(file.is_file(), "{}", file.display());
}

#[test]
fn an_empty_xdg_cache_home_is_as_if_it_were_not_set() {
    let world = world("xdg-empty");
    world.topic(REPO, "");
    let home = world.cache.join("home");

    let output = refs_run(
        &world.dir,
        &["show", "demo"],
        &Setup {
            maps: vec![(REPO, &world.origin)],
            env: vec![
                ("XDG_CACHE_HOME", String::new()),
                ("HOME", home.display().to_string()),
            ],
            removed: vec!["LOCALAPPDATA"],
        },
    );

    ok(&output);
    assert!(home.join(".cache/botica/refs").is_dir());
}

#[cfg(not(windows))]
#[test]
fn local_app_data_is_for_windows_only() {
    let world = world("local-app-data");
    world.topic(REPO, "");
    let home = world.cache.join("home");
    let local = world.cache.join("local");

    let output = refs_run(
        &world.dir,
        &["show", "demo"],
        &Setup {
            maps: vec![(REPO, &world.origin)],
            env: vec![
                ("HOME", home.display().to_string()),
                ("LOCALAPPDATA", local.display().to_string()),
            ],
            removed: vec!["XDG_CACHE_HOME"],
        },
    );

    ok(&output);
    assert!(home.join(".cache/botica/refs").is_dir());
    assert!(!local.exists());
}

#[test]
fn without_a_place_for_the_cache_the_command_asks_for_cache_dir() {
    let world = world("no-home");
    world.topic(REPO, "");

    let output = refs_run(
        &world.dir,
        &["show", "demo"],
        &Setup {
            maps: vec![(REPO, &world.origin)],
            env: vec![],
            removed: vec!["XDG_CACHE_HOME", "HOME", "LOCALAPPDATA"],
        },
    );

    let text = failed(&output, 2);
    assert!(text.contains("--cache-dir"), "{text}");
}

#[test]
fn a_reference_that_the_cache_holds_needs_no_git() {
    let world = world("no-git-cached");
    world.topic(REPO, "");
    ok(&world.run(REPO, &["show", "demo"]));
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

    ok(&output);
}
