//! `agents-md-refs-exist`: each `@<path>` reference of an `AGENTS.md` names a file of the project.

use botica_core::agents_refs::{check, references, Source};
use botica_core::Severity;

fn paths(text: &str) -> Vec<&str> {
    references(text).into_iter().map(|r| r.path).collect()
}

fn reported(files: &[&str], text: &str, at: &str) -> Vec<String> {
    let sources = [Source { path: at, text }];
    let exists = |path: &str| files.contains(&path) || path.is_empty();
    check(&sources, &exists, &|_| false, Severity::Error)
        .into_iter()
        .map(|finding| finding.message)
        .collect()
}

#[test]
fn a_reference_has_a_slash_or_the_extension_of_a_file() {
    let text =
        "See @docs/a.md, `@README.md` and (@src/).\nTag @latest, @octocat, @pytest.fixture.\n";
    assert_eq!(paths(text), ["docs/a.md", "README.md", "src/"]);
}

#[test]
fn an_email_a_url_a_version_and_a_fenced_block_hold_no_reference() {
    let text = "Mail a@b.md or https://x.com/@u/s.md.\n`pnpm add @scope/pkg@1.0`\n\
                ```bash\ncat @docs/a.md\n```\n~~~\n@docs/b.md\n~~~\nAfter @docs/c.md\n";
    assert_eq!(paths(text), ["docs/c.md"]);
}

#[test]
fn the_range_covers_the_at_sign_and_the_path() {
    let found = references("x `@a/b.md`\n");
    assert_eq!((found[0].start, found[0].end), (3, 10));
}

#[test]
fn a_reference_resolves_from_its_folder_or_from_the_root() {
    let files = ["pkg/notes.md", "GLOSSARY.md", "docs/a.md"];
    let text = "@notes.md @../GLOSSARY.md @docs/a.md @missing.md\n";
    let found = reported(&files, text, "pkg/AGENTS.md");
    assert_eq!(found.len(), 1);
    assert!(
        found[0].starts_with("`@missing.md` names no file"),
        "{found:?}"
    );
}

#[test]
fn a_package_scope_whose_first_folder_is_absent_is_not_a_path() {
    let files = ["docs/a.md", "docs"];
    let found = reported(&files, "@types/node @docs/gone @nope/x.md\n", "AGENTS.md");
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("`@docs/gone`"));
    assert!(found[1].starts_with("`@nope/x.md`"));
}

#[test]
fn a_path_that_gitignore_leaves_out_or_outside_the_project_is_not_reported() {
    let sources = [Source {
        path: "AGENTS.md",
        text: "@.agents/skills/a/SKILL.md @~/notes.md @/etc/hosts.txt @../../up.md\n",
    }];
    let ignored = |path: &str| path.starts_with(".agents/skills/");
    assert!(check(&sources, &|_| false, &ignored, Severity::Error).is_empty());
}
