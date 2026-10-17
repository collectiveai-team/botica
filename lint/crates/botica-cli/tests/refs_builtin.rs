//! The topics that the rules of botica cite come with the binary, so `botica refs` reads them in
//! any project.

mod common;

use std::path::{Path, PathBuf};

use common::builtin::{counts, embedded, fill_cache};
use common::git_remote::{refs_at, refs_run, Origin, Setup};
use common::refs::{add_topic, failed, ok, refs};
use common::{project_in, write};

const BOTICA: &str = "github.com/collectiveai-team/botica";

fn cache(name: &str) -> PathBuf {
    let cache = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("refs-builtin-cache")
        .join(name);
    let _ = std::fs::remove_dir_all(&cache);
    cache
}

fn with_cache(args: &[&str], cache: &Path) -> Vec<String> {
    let mut all: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    all.extend(["--cache-dir".to_owned(), cache.display().to_string()]);
    all
}

fn run(dir: &Path, args: &[&str], cache: &Path) -> std::process::Output {
    let all = with_cache(args, cache);
    let all: Vec<&str> = all.iter().map(String::as_str).collect();
    refs(dir, &all)
}

const OWN_TOPIC: &str = "title = \"The logging of this project\"\n\n[[references]]\n\
                         path = \"src/log.py\"\nlines = \"1-2\"\nnote = \"our logger\"\n";

/// A project whose `references/` holds a topic with the id of a built-in one.
fn project_with_own_logging(name: &str, config: Option<&str>) -> PathBuf {
    let dir = project_in("refs-builtin", name, config);
    write(&dir, "src/log.py", "def log():\n    pass\n");
    add_topic(&dir, "structured-logging", OWN_TOPIC);
    dir
}

#[test]
fn a_rule_of_botica_shows_its_topic_in_a_project_that_has_none() {
    let dir = project_in("refs-builtin", "show-rule", None);
    let cache = cache("show-rule");
    fill_cache(&cache);

    let text = ok(&run(&dir, &["show", "log-no-print"], &cache));

    let embedded = embedded();
    let topic = embedded.topic("structured-logging").expect("the topic");
    assert!(text.contains("# structured-logging: "), "{text}");
    assert!(
        text.contains(topic.readme.lines().next().expect("a line")),
        "{text}"
    );
    for reference in &topic.references {
        assert!(text.contains(&reference.note), "{}: {text}", reference.note);
    }
    assert!(text.contains("license: MIT"), "{text}");
}

#[test]
fn a_topic_of_botica_is_shown_by_its_id() {
    let dir = project_in("refs-builtin", "show-id", None);
    let cache = cache("show-id");
    fill_cache(&cache);

    let text = ok(&run(&dir, &["show", "in-memory-adapters"], &cache));

    assert!(text.contains("# in-memory-adapters: "), "{text}");
}

#[test]
fn list_has_the_built_in_topics_and_says_where_each_comes_from() {
    let dir = project_in("refs-builtin", "list", None);
    write(&dir, "src/app.py", "x = 1\n");
    add_topic(
        &dir,
        "mine",
        "title = \"Mine\"\n\n[[references]]\npath = \"src/app.py\"\nnote = \"n\"\n",
    );

    let text = ok(&refs(&dir, &["list"]));

    let block = |id: &str| {
        text.split("\n\n")
            .find(|block| block.starts_with(id))
            .unwrap_or_else(|| panic!("no block for {id}:\n{text}"))
            .to_owned()
    };
    assert!(
        block("structured-logging").contains("  from: built-in"),
        "{text}"
    );
    assert!(
        block("structured-logging").contains("log-no-print"),
        "{text}"
    );
    assert!(
        block("in-memory-adapters").contains("  from: built-in"),
        "{text}"
    );
    assert!(block("mine").contains("  from: project"), "{text}");
}

#[test]
fn check_does_not_fetch_the_references_of_the_built_in_topics() {
    let dir = project_in("refs-builtin", "check-quiet", None);
    let empty = cache("check-quiet");
    std::fs::create_dir_all(&empty).expect("create the folder");

    let output = refs_run(
        &dir,
        &["check"],
        &Setup {
            env: vec![("PATH", empty.display().to_string())],
            removed: vec!["HOME", "XDG_CACHE_HOME", "LOCALAPPDATA"],
            ..Setup::default()
        },
    );

    assert_eq!(ok(&output), "0 topics and 0 references resolve\n");
}

#[test]
fn check_all_resolves_the_references_of_the_built_in_topics_too() {
    let dir = project_in("refs-builtin", "check-all", None);
    let cache = cache("check-all");
    fill_cache(&cache);

    let text = ok(&run(&dir, &["check", "--all"], &cache));

    let (topics, references) = counts();
    assert_eq!(
        text,
        format!("{topics} topics and {references} references resolve\n")
    );
}

#[test]
fn check_all_fails_for_a_built_in_reference_that_cannot_be_fetched() {
    let dir = project_in("refs-builtin", "check-all-cold", None);
    let cache = cache("check-all-cold");
    let origin = Origin::new("refs-builtin-empty");
    origin.commit(&[("README.md", b"nothing here\n")]);
    let all = with_cache(&["check", "--all"], &cache);
    let all: Vec<&str> = all.iter().map(String::as_str).collect();

    let output = refs_at(&dir, &all, BOTICA, &origin);

    let text = failed(&output, 1);
    assert!(
        text.contains("references/structured-logging/topic.toml"),
        "{text}"
    );
    assert!(text.contains(&format!("cannot fetch {BOTICA}@")), "{text}");
}

#[test]
fn a_project_topic_with_the_id_of_a_built_in_one_is_an_error() {
    let dir = project_with_own_logging("collision", None);

    let text = failed(&refs(&dir, &["check"]), 1);

    assert!(
        text.contains("references/structured-logging/topic.toml"),
        "{text}"
    );
    assert!(
        text.contains("the id `structured-logging` is the one of a built-in topic"),
        "{text}"
    );
}

#[test]
fn the_configuration_lets_a_project_topic_replace_a_built_in_one() {
    let dir = project_with_own_logging(
        "replaced",
        Some("[references]\nreplace = [\"structured-logging\"]\n"),
    );

    assert_eq!(
        ok(&refs(&dir, &["check"])),
        "1 topic and 1 reference resolve\n"
    );
    let list = ok(&refs(&dir, &["list"]));
    let block = list
        .split("\n\n")
        .find(|block| block.starts_with("structured-logging"))
        .expect("the block");
    assert!(block.contains("  from: project"), "{list}");
    let shown = ok(&refs(&dir, &["show", "log-no-print"]));
    assert!(shown.contains("our logger"), "{shown}");
    assert!(shown.contains("def log():"), "{shown}");
}

#[test]
fn replace_must_name_a_built_in_topic_that_the_project_defines() {
    let ghost = project_in(
        "refs-builtin",
        "replace-ghost",
        Some("[references]\nreplace = [\"ghost\"]\n"),
    );
    let text = failed(&refs(&ghost, &["check"]), 1);
    assert!(
        text.lines()
            .any(|line| line.starts_with("[references] replace: `ghost` is not a built-in topic")),
        "{text}"
    );

    let missing = project_in(
        "refs-builtin",
        "replace-missing",
        Some("[references]\nreplace = [\"structured-logging\"]\n"),
    );
    let text = failed(&refs(&missing, &["check"]), 1);
    let expected = "[references] replace: `structured-logging` is a built-in topic, and \
                    `references/` has no topic with that id";
    assert!(text.lines().any(|line| line == expected), "{text}");
}
