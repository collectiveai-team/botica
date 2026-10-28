//! `botica-version-aligned`: the hooks, the workflows and the parameter `botica-version` follow
//! the `rev` of the mirror of botica.

use botica_core::botica_version::{
    check, mirror_version, pins, pre_commit_rev, reads, Param, Source,
};
use botica_core::Severity;

const MIRROR: &str =
    "[[repos]]\nrepo = \"https://github.com/collectiveai-team/botica-pre-commit\"\n\
                      rev = \"v0.17.1\"\n";

fn versions(text: &str) -> Vec<&str> {
    pins(text).into_iter().map(|pin| pin.version).collect()
}

fn reported(files: &[(&str, &str)]) -> Vec<String> {
    let sources: Vec<Source<'_>> = files
        .iter()
        .map(|(path, text)| Source { path, text })
        .collect();
    check(&sources, None, Severity::Warning)
        .into_iter()
        .map(|finding| format!("{}:{}: {}", finding.path, finding.line, finding.message))
        .collect()
}

#[test]
fn the_rule_reads_prek_toml_and_the_workflows_at_the_root() {
    assert!(reads("prek.toml"));
    assert!(reads(".github/workflows/abide.yml"));
    assert!(reads(".github/workflows/ci.yaml"));
    assert!(reads(".pre-commit-config.yaml"));
    assert!(!reads("docs/prek.toml"));
    assert!(!reads(".github/workflows/scripts/run.yml"));
    assert!(!reads(".github/dependabot.yml"));
}

#[test]
fn a_pin_is_a_version_that_starts_with_a_digit_outside_a_comment() {
    let text = "run: uvx botica@0.17.1 lint && uvx botica@latest rules\n\
                # was uvx botica@0.16.1\n\
                uses: owner/botica@v1\n\
                entry = \"uvx botica@0.18.0rc1 lint\"\n\
                x: uvx botica@{{ param.botica-version }} lint; pre-botica@1.0\n";
    assert_eq!(versions(text), ["0.17.1", "0.18.0rc1"]);
}

#[test]
fn the_range_covers_the_name_and_the_version() {
    let found = pins("run: uvx botica@0.17.1 lint\n");
    assert_eq!((found[0].start, found[0].end), (9, 22));
}

#[test]
fn the_mirror_gives_its_rev_without_the_v() {
    assert_eq!(mirror_version(MIRROR).as_deref(), Some("0.17.1"));
    let with_git = MIRROR.replace("pre-commit\"", "pre-commit.git\"");
    assert_eq!(mirror_version(&with_git).as_deref(), Some("0.17.1"));
    assert_eq!(mirror_version(&MIRROR.replace("v0.17.1", "main")), None);
    assert_eq!(mirror_version("[[repos]]\nrepo = \"local\"\n"), None);
    assert_eq!(mirror_version("not toml ="), None);
}

#[test]
fn a_pin_that_differs_from_the_mirror_is_reported_where_it_is() {
    let prek = format!(
        "[[repos]]\nrepo = \"local\"\n[[repos.hooks]]\nid = \"a\"\n\
         entry = \"uvx botica@0.16.1 lint\"\n{MIRROR}"
    );
    let found = reported(&[
        ("prek.toml", &prek),
        (".github/workflows/a.yml", "run: uvx botica@0.17.1 lint\n"),
        (
            ".github/workflows/b.yml",
            "x: 1\nrun: uvx botica@0.16.1 lint\n",
        ),
    ]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        found[0],
        "prek.toml:5: This line runs botica 0.16.1, and the `rev` of the mirror \
         `collectiveai-team/botica-pre-commit` in prek.toml is `v0.17.1`: write `botica@0.17.1` \
         here, so that the hooks and the workflows run one version of botica"
    );
    assert!(
        found[1].starts_with(".github/workflows/b.yml:2: "),
        "{found:?}"
    );
}

#[test]
fn without_the_mirror_the_newest_version_is_the_reference() {
    let found = reported(&[
        (
            "prek.toml",
            "[[repos]]\nrepo = \"local\"\n# uvx botica@0.1.0\n",
        ),
        (".github/workflows/a.yml", "run: uvx botica@0.9.0 lint\n"),
        (
            ".github/workflows/b.yml",
            "run: uvx botica@0.10.0rc1 lint\n",
        ),
        (".github/workflows/c.yml", "run: uvx botica@0.10.0 lint\n"),
    ]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found[0].starts_with(
            ".github/workflows/a.yml:1: This line runs botica 0.9.0, and the newest version that \
             the project names is 0.10.0"
        ),
        "{found:?}"
    );
    assert!(found[1].contains("runs botica 0.10.0rc1"), "{found:?}");
}

#[test]
fn one_version_everywhere_or_no_pin_is_no_finding() {
    let prek = format!("[[repos]]\nrepo = \"local\"\nentry = \"uvx botica@0.17.1\"\n{MIRROR}");
    assert!(reported(&[
        ("prek.toml", &prek),
        (".github/workflows/a.yml", "run: uvx botica@0.17.1 lint\n"),
    ])
    .is_empty());
    assert!(reported(&[("prek.toml", MIRROR)]).is_empty());
    assert!(reported(&[(".github/workflows/a.yml", "run: cargo run -p botica-cli\n")]).is_empty());
}

const PRE_COMMIT: &str = "repos:\n  - repo: local\n    hooks:\n      - id: a\n        \
                          entry: uvx botica@0.16.1 lint\n  \
                          - repo: https://github.com/collectiveai-team/botica-pre-commit\n    \
                          hooks:\n      - id: botica\n    rev: 'v0.16.1'\n";

#[test]
fn the_rev_of_the_mirror_in_pre_commit_is_the_one_of_its_item() {
    let rev = pre_commit_rev(PRE_COMMIT).expect("the rev");
    assert_eq!(rev.version, "0.16.1");
    assert_eq!(&PRE_COMMIT[rev.start..rev.end], "v0.16.1");
    let other = "repos:\n  - repo: https://github.com/collectiveai-team/botica-pre-commit\n  \
                 - repo: https://github.com/astral-sh/ruff-pre-commit\n    rev: v0.15.0\n";
    assert_eq!(pre_commit_rev(other), None);
}

#[test]
fn without_a_mirror_in_prek_toml_the_one_of_pre_commit_is_the_reference() {
    let found = reported(&[
        (".pre-commit-config.yaml", PRE_COMMIT),
        (".github/workflows/a.yml", "run: uvx botica@0.17.1 lint\n"),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with(
            ".github/workflows/a.yml:1: This line runs botica 0.17.1, and the `rev` of the mirror \
             `collectiveai-team/botica-pre-commit` in .pre-commit-config.yaml is `v0.16.1`"
        ),
        "{found:?}"
    );
}

#[test]
fn with_both_mirrors_the_rev_of_pre_commit_follows_prek_toml() {
    let found = reported(&[
        ("prek.toml", MIRROR),
        (".pre-commit-config.yaml", PRE_COMMIT),
    ]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with(".pre-commit-config.yaml:5: This line runs botica 0.16.1"));
    assert!(
        found[1].starts_with(
            ".pre-commit-config.yaml:9: This `rev` installs botica 0.16.1, and the `rev` of the \
             mirror `collectiveai-team/botica-pre-commit` in prek.toml is `v0.17.1`: write \
             `v0.17.1` here"
        ),
        "{found:?}"
    );
}

fn param_found(path: &str, text: &str, value: &str) -> Vec<String> {
    let sources = [Source {
        path: "prek.toml",
        text: MIRROR,
    }];
    let param = Param { path, text, value };
    check(&sources, Some(&param), Severity::Warning)
        .into_iter()
        .map(|finding| {
            format!(
                "{}:{}:{}-{}: {}",
                finding.path, finding.line, finding.column, finding.end_column, finding.message
            )
        })
        .collect()
}

#[test]
fn the_parameter_botica_version_is_reported_at_its_value() {
    let found = param_found(
        "botica.toml",
        "select = []\n\n[recipes.params]\nbotica-version = \"0.16.1\"\n",
        "0.16.1",
    );
    assert_eq!(
        found,
        ["botica.toml:4:18-26: The parameter `botica-version = \"0.16.1\"` of the recipes makes \
          the next `botica recipe install` write `botica@0.16.1` again, and the `rev` of the mirror \
          `collectiveai-team/botica-pre-commit` in prek.toml is `v0.17.1`: set it to `0.17.1`, or \
          remove it so that `install` writes the version of the botica that runs it"]
    );
    let pyproject = "[tool.botica.recipes]\nparams = { \"botica-version\" = '0.16.1' }\n";
    let found = param_found("pyproject.toml", pyproject, "0.16.1");
    assert!(
        found[0].starts_with("pyproject.toml:2:31-39: "),
        "{found:?}"
    );
    assert!(param_found(
        "botica.toml",
        "[recipes.params]\nbotica-version = \"0.17.1\"\n",
        "0.17.1"
    )
    .is_empty());
}
