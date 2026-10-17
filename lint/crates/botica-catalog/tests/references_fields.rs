//! What `topic.toml` accepts in a reference: a page or a file, the commit and the repository.

mod common;

use common::topics::{load, rejected, topic, EXTERNAL, INTERNAL, PAGE, SHA};

#[test]
fn a_page_reference_has_a_url_and_a_note_and_no_file() {
    let loaded = load(vec![topic("structured-logging", PAGE)]).expect("loads");

    let reference = &loaded
        .topic("structured-logging")
        .expect("topic")
        .references[0];
    assert!(reference.is_page());
    assert_eq!(
        reference.url.as_deref(),
        Some("https://www.structlog.org/en/stable/getting-started.html")
    );
    assert_eq!(reference.path, None);
    assert_eq!(reference.note, "how to configure structlog");
    assert!(!reference.is_external());
}

#[test]
fn a_reference_needs_a_path_or_a_url_and_not_both() {
    let neither = PAGE.replace(
        "url = \"https://www.structlog.org/en/stable/getting-started.html\"\n",
        "",
    );
    let message = rejected(vec![topic("structured-logging", &neither)]);
    assert!(message.contains("needs a `path` or a `url`"), "{message}");

    let both = format!("{PAGE}path = \"docs/a.md\"\n");
    let message = rejected(vec![topic("structured-logging", &both)]);
    assert!(
        message.contains("`path` and `url` exclude each other"),
        "{message}"
    );
}

#[test]
fn a_url_must_be_an_http_address() {
    for bad in [
        "ftp://example.com/a",
        "www.example.com/a",
        "javascript:alert(1)",
        "https://",
        "http://",
        "https://example.com/a b",
        "",
    ] {
        let toml = PAGE.replace(
            "https://www.structlog.org/en/stable/getting-started.html",
            bad,
        );
        let message = rejected(vec![topic("structured-logging", &toml)]);
        assert!(
            message.contains("`url` must be an `https://` or `http://` address"),
            "{bad}: {message}"
        );
    }
    for good in ["http://example.com/a", "https://example.com"] {
        let toml = PAGE.replace(
            "https://www.structlog.org/en/stable/getting-started.html",
            good,
        );
        load(vec![topic("structured-logging", &toml)]).unwrap_or_else(|e| panic!("{good}: {e}"));
    }
}

#[test]
fn a_page_reference_takes_none_of_the_fields_of_a_file() {
    let fields = [
        "lines = \"1-2\"",
        "anchor = \"x\"",
        "repo = \"github.com/acme/app\"",
        &format!("rev = \"{SHA}\""),
        "license = \"MIT\"",
    ];
    for field in fields {
        let message = rejected(vec![topic("t", &format!("{PAGE}{field}\n"))]);
        let name = field.split(' ').next().expect("a field name");
        assert!(
            message.contains(&format!("`{name}` does not apply to a `url` reference")),
            "{field}: {message}"
        );
        assert!(
            !message.contains(" needs a "),
            "a page reference reports only the field that does not apply: {field}: {message}"
        );
    }
}

#[test]
fn the_docs_of_a_topic_are_no_longer_a_field() {
    let toml = format!("docs = [\"https://example.com\"]\n{INTERNAL}");
    let message = rejected(vec![topic("structured-logging", &toml)]);
    assert!(message.contains("docs"), "{message}");
}

#[test]
fn a_rev_is_a_full_commit_sha() {
    let sha256 = "a".repeat(64);
    for good in [SHA, sha256.as_str()] {
        let toml = EXTERNAL.replace(SHA, good);
        load(vec![topic("t", &toml)]).unwrap_or_else(|e| panic!("{good}: {e}"));
    }
    let bad = [
        "3f2a9c1",
        "main",
        "v1.2.0",
        &SHA.to_uppercase(),
        &format!("{SHA}0"),
        &SHA[..39],
        &"g".repeat(40),
        "",
    ];
    for bad in bad {
        let toml = EXTERNAL.replace(SHA, bad);
        let message = rejected(vec![topic("t", &toml)]);
        assert!(
            message.contains("`rev` must be a full commit SHA"),
            "{bad}: {message}"
        );
    }
}

#[test]
fn a_repo_is_a_host_and_a_path_without_a_scheme() {
    for good in [
        "github.com/acme/app",
        "gitlab.example.com/group/sub/app",
        "git.example.org/a_b/c.d-e",
    ] {
        let toml = EXTERNAL.replace("github.com/acme/app", good);
        load(vec![topic("t", &toml)]).unwrap_or_else(|e| panic!("{good}: {e}"));
    }
    for bad in [
        "https://github.com/acme/app",
        "git@github.com:acme/app",
        "github.com",
        "github.com/",
        "github.com//app",
        "github.com/acme/",
        "github.com/../app",
        "github.com/./app",
        "github.com/acme/-app",
        "-host.com/acme/app",
        ".host.com/acme/app",
        "git hub.com/acme/app",
        "github.com/acme/my app",
        "user@github.com/acme/app",
        "github.com:443/acme/app",
        "/acme/app",
        "",
    ] {
        let toml = EXTERNAL.replace("github.com/acme/app", bad);
        let message = rejected(vec![topic("t", &toml)]);
        assert!(
            message.contains("`repo` must be `<host>/<path>`"),
            "{bad}: {message}"
        );
    }
}

#[test]
fn rejects_a_repo_without_a_rev_and_a_rev_without_a_repo() {
    let no_rev = EXTERNAL.replace(&format!("rev = \"{SHA}\"\n"), "");
    let message = rejected(vec![topic("structured-logging", &no_rev)]);
    assert!(message.contains("`repo` needs a `rev`"), "{message}");

    let no_repo = EXTERNAL.replace("repo = \"github.com/acme/app\"\n", "");
    let message = rejected(vec![topic("structured-logging", &no_repo)]);
    assert!(message.contains("`rev` needs a `repo`"), "{message}");
}

#[test]
fn rejects_an_external_reference_without_a_license() {
    let no_license = EXTERNAL.replace("license = \"MIT\"\n", "");
    let message = rejected(vec![topic("structured-logging", &no_license)]);
    assert!(
        message.contains("external reference needs a `license`"),
        "{message}"
    );
}

#[test]
fn rejects_a_license_on_an_internal_reference() {
    let message = rejected(vec![topic(
        "structured-logging",
        &format!("{INTERNAL}license = \"MIT\"\n"),
    )]);
    assert!(message.contains("`license` needs a `repo`"), "{message}");
}

#[test]
fn a_page_reference_may_quote_the_passage_that_matters() {
    let toml = format!("{PAGE}quote = \"Call configure() once.\"\n");

    let loaded = load(vec![topic("t", &toml)]).expect("loads");

    let reference = &loaded.topic("t").expect("topic").references[0];
    assert_eq!(reference.quote.as_deref(), Some("Call configure() once."));
}

#[test]
fn a_quote_belongs_to_a_page_and_is_not_empty() {
    let message = rejected(vec![topic("t", &format!("{INTERNAL}quote = \"x\"\n"))]);
    assert!(
        message.contains("`quote` does not apply to a file reference"),
        "{message}"
    );

    for empty in ["", "  \\n "] {
        let message = rejected(vec![topic("t", &format!("{PAGE}quote = \"{empty}\"\n"))]);
        assert!(message.contains("`quote` is empty"), "{empty:?}: {message}");
    }
}

fn first(toml: &str) -> botica_catalog::Reference {
    load(vec![topic("t", toml)])
        .expect("loads")
        .topic("t")
        .expect("topic")
        .references[0]
        .clone()
}

#[test]
fn a_file_without_lines_is_a_whole_file_and_the_reference_suggests_lines() {
    let whole = first(&INTERNAL.replace("lines = \"18-30\"\n", ""));
    let advice = whole.suggestion().expect("a suggestion");
    assert!(advice.contains("points at the whole file"), "{advice}");
    assert!(advice.contains("`lines`"), "{advice}");

    assert_eq!(first(INTERNAL).suggestion(), None);
    assert_eq!(
        first(EXTERNAL)
            .suggestion()
            .map(|a| a.contains("whole file")),
        Some(true)
    );
}

#[test]
fn a_page_without_a_fragment_or_a_quote_is_a_whole_page_and_the_reference_suggests_one() {
    let advice = first(PAGE).suggestion().expect("a suggestion");
    assert!(advice.contains("points at the whole page"), "{advice}");
    assert!(advice.contains("`#fragment`"), "{advice}");
    assert!(advice.contains("`quote`"), "{advice}");

    let fragment = PAGE.replace("getting-started.html", "getting-started.html#configuration");
    assert_eq!(first(&fragment).suggestion(), None);
    assert_eq!(
        first(&format!("{PAGE}quote = \"a passage\"\n")).suggestion(),
        None
    );
    let empty_fragment = PAGE.replace("getting-started.html", "getting-started.html#");
    assert!(first(&empty_fragment).suggestion().is_some());
}
