//! `external-config-weakened`: a finding on the line of each value that holds less than the
//! fragment of an active external rule.

use botica_core::external_config::Weakening;
use botica_core::external_weakened::check;
use botica_core::Severity;

fn weakening(rule: &str, file: &str, key: &str, wanted: &str, found: &str) -> Weakening {
    Weakening {
        rule: rule.to_owned(),
        tool: "tool".to_owned(),
        file: file.to_owned(),
        key: key.to_owned(),
        wanted: wanted.to_owned(),
        found: found.to_owned(),
    }
}

/// `line:column message` of each finding, over the files `files`.
fn found(weakenings: &[Weakening], files: &[(&str, &str)]) -> Vec<String> {
    let read = |path: &str| {
        files
            .iter()
            .find(|(name, _)| *name == path)
            .map(|(_, text)| (*text).to_owned())
    };
    check(weakenings, read, Severity::Warning)
        .iter()
        .map(|diagnostic| {
            let at = if diagnostic.file_level {
                "file".to_owned()
            } else {
                format!("{}:{}", diagnostic.line, diagnostic.column)
            };
            format!("{} {at} {}", diagnostic.path, diagnostic.message)
        })
        .collect()
}

const PYPROJECT: &str = "[project]\nname = \"app\"\n\n[tool.pyrefly]\npreset = \"basic\"\n\n\
                         [tool.pyrefly.errors]\nexplicit-any = \"warn\"\n\n[tool.uv]\n\
                         exclude-newer = \"3 days\"\n\n[tool.ruff.lint]\nselect = [\"ALL\"]\n\
                         ignore = [\n  \"D\",\n  \"DOC\",\n]\n";

#[test]
fn the_finding_sits_on_the_key_and_names_the_value_the_rule_and_what_it_asks() {
    let weakenings = [
        weakening(
            "pyrefly-strict",
            "pyproject.toml",
            "preset",
            "\"strict\"",
            "\"basic\"",
        ),
        weakening(
            "uv-cooldown",
            "pyproject.toml",
            "exclude-newer",
            "\"7 days\"",
            "\"3 days\"",
        ),
    ];

    assert_eq!(
        found(&weakenings, &[("pyproject.toml", PYPROJECT)]),
        [
            "pyproject.toml 5:1 `preset = \"basic\"` in pyproject.toml weakens what \
             `pyrefly-strict` asks for (`strict`): give the reason in a \
             `botica: ignore[external-config-weakened] -- <reason>` comment on that line, or \
             switch the rule off",
            "pyproject.toml 11:1 `exclude-newer = \"3 days\"` in pyproject.toml weakens what \
             `uv-cooldown` asks for (`7 days`): give the reason in a \
             `botica: ignore[external-config-weakened] -- <reason>` comment on that line, or \
             switch the rule off",
        ]
    );
}

#[test]
fn a_key_of_a_table_of_the_tool_and_a_code_of_ruff_in_ignore_are_found() {
    let weakenings = [
        weakening(
            "pyrefly-strict",
            "pyproject.toml",
            "errors.explicit-any",
            "\"error\"",
            "\"warn\"",
        ),
        weakening(
            "doc-comments",
            "pyproject.toml",
            "lint.select",
            "\"DOC102\"",
            "ignore = [\"DOC\"]",
        ),
    ];

    let lines: Vec<String> = found(&weakenings, &[("pyproject.toml", PYPROJECT)])
        .iter()
        .map(|line| line.split(" `").next().unwrap_or_default().to_owned())
        .collect();

    assert_eq!(lines, ["pyproject.toml 8:1", "pyproject.toml 17:3"]);
    assert!(found(&weakenings, &[("pyproject.toml", PYPROJECT)])[1].starts_with(
        "pyproject.toml 17:3 `ignore = [\"DOC\"]` in pyproject.toml weakens what `doc-comments` \
         asks for (`DOC102`)"
    ));
}

#[test]
fn yaml_and_npmrc_show_the_value_as_the_file_writes_it() {
    let weakenings = [
        weakening(
            "pnpm-cooldown",
            "pnpm-workspace.yaml",
            "minimumReleaseAge",
            "10080",
            "0",
        ),
        weakening(
            "npm-cooldown",
            ".npmrc",
            "min-release-age",
            "\"7\"",
            "\"3\"",
        ),
    ];
    let files = [
        (
            "pnpm-workspace.yaml",
            "packages:\n  - app\nminimumReleaseAge: 0\n",
        ),
        (".npmrc", "fund=false\nmin-release-age=3\n"),
    ];

    let found = found(&weakenings, &files);

    assert!(
        found[0].starts_with(
            "pnpm-workspace.yaml 3:1 `minimumReleaseAge: 0` in pnpm-workspace.yaml weakens what \
             `pnpm-cooldown` asks for (`10080`)"
        ),
        "{found:?}"
    );
    assert!(
        found[1].starts_with(
            ".npmrc 2:1 `min-release-age=3` in .npmrc weakens what `npm-cooldown` asks for (`7`)"
        ),
        "{found:?}"
    );
}

#[test]
fn in_an_ini_file_of_pytest_the_message_asks_for_a_comment_on_a_line_of_its_own() {
    let weakenings = [weakening(
        "pytest-strict",
        "pytest.ini",
        "xfail_strict",
        "\"true\"",
        "\"false\"",
    )];

    assert_eq!(
        found(
            &weakenings,
            &[("pytest.ini", "[pytest]\nxfail_strict = false\n")]
        ),
        [
            "pytest.ini 2:1 `xfail_strict=false` in pytest.ini weakens what `pytest-strict` asks \
          for (`true`): give the reason in a \
          `botica: ignore-file[external-config-weakened] -- <reason>` comment on a line of its \
          own, or switch the rule off"
        ]
    );
}

#[test]
fn the_key_whose_value_matches_wins_and_a_longer_key_is_another_key() {
    let text = "[tool.other]\npreset = \"x\"\nmy-preset = \"basic\"\n\n[tool.pyrefly]\n\
                preset = \"basic\"\n";
    let weakenings = [weakening(
        "pyrefly-strict",
        "pyproject.toml",
        "preset",
        "\"strict\"",
        "\"basic\"",
    )];

    assert!(found(&weakenings, &[("pyproject.toml", text)])[0].starts_with("pyproject.toml 6:1"));
}

#[test]
fn a_value_that_botica_does_not_find_is_a_finding_about_the_whole_file() {
    let weakenings = [weakening(
        "pyrefly-strict",
        "pyrefly.toml",
        "preset",
        "\"strict\"",
        "\"basic\"",
    )];

    assert!(found(
        &weakenings,
        &[("pyrefly.toml", "extends = \"base.toml\"\n")]
    )[0]
    .starts_with("pyrefly.toml file "));
    assert!(found(&weakenings, &[])[0].starts_with("pyrefly.toml file "));
}
