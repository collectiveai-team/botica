//! `external-config` reads the settings of a tool that botica knows from the files the tool reads
//! them from, in its order, and accepts what the tool accepts: another key, another file, and the
//! same setting written another way. A value that the project sets weaker on purpose is not a
//! finding: `weakened` gives it.

use std::collections::HashMap;

use botica_core::external_config::{check, in_use, weakened, Requirement, Weakening};
use botica_core::{Diagnostic, Severity};

const RUFF_FRAGMENT: &str = "[tool.ruff.lint]\nselect = [\"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n";

fn requirement<'a>(tool: &'a str, target: &'a str, fragment: &'a str) -> Requirement<'a> {
    Requirement {
        rule: "the-rule",
        tool,
        target,
        fragment,
    }
}

fn findings(requirement: Requirement<'_>, files: &[(&str, &str)]) -> Vec<Diagnostic> {
    let files: HashMap<&str, &str> = files.iter().copied().collect();
    check(
        &[requirement],
        |path| files.get(path).map(|text| (*text).to_owned()),
        Severity::Error,
    )
}

fn weakenings(requirement: Requirement<'_>, files: &[(&str, &str)]) -> Vec<Weakening> {
    let files: HashMap<&str, &str> = files.iter().copied().collect();
    weakened(&[requirement], |path| {
        files.get(path).map(|text| (*text).to_owned())
    })
}

/// The key, the value of the fragment and the value of the settings of each weakening.
fn weaker(requirement: Requirement<'_>, files: &[(&str, &str)]) -> Vec<(String, String, String)> {
    weakenings(requirement, files)
        .into_iter()
        .map(|weakening| (weakening.key, weakening.wanted, weakening.found))
        .collect()
}

fn ruff(files: &[(&str, &str)]) -> Vec<Diagnostic> {
    findings(requirement("ruff", "pyproject.toml", RUFF_FRAGMENT), files)
}

/// The path of the one finding, or `None` when there is none.
fn reported(found: &[Diagnostic]) -> Option<&str> {
    assert!(found.len() <= 1, "{found:?}");
    found.first().map(|finding| finding.path.as_str())
}

#[test]
fn ruff_holds_the_codes_in_select_or_in_extend_select() {
    for pyproject in [
        "[tool.ruff.lint]\nselect = [\"E\", \"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n",
        "[tool.ruff.lint]\nselect = [\"E\"]\nextend-select = [\"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n",
        "[tool.ruff.lint]\nselect = [\"ALL\"]\n",
        "[tool.ruff]\nselect = [\"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n",
    ] {
        assert_eq!(reported(&ruff(&[("pyproject.toml", pyproject)])), None, "{pyproject}");
    }
}

#[test]
fn a_family_that_the_project_ignores_is_its_decision_and_is_not_reported() {
    let ignored = "[tool.ruff.lint]\nselect = [\"E\", \"ASYNC\", \"SLOT\"]\nignore = [\"A\"]\n\
                   extend-ignore = [\"BLE\"]\n";
    assert_eq!(reported(&ruff(&[("pyproject.toml", ignored)])), None);
    assert_eq!(
        weaker(
            requirement("ruff", "pyproject.toml", RUFF_FRAGMENT),
            &[("pyproject.toml", ignored)]
        ),
        [
            (
                "lint.select".to_owned(),
                "\"A\"".to_owned(),
                "ignore = [\"A\"]".to_owned()
            ),
            (
                "lint.select".to_owned(),
                "\"BLE\"".to_owned(),
                "ignore = [\"BLE\"]".to_owned()
            ),
        ]
    );

    let one_code = "[tool.ruff.lint]\nselect = [\"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n\
                    ignore = [\"A005\"]\n";
    assert_eq!(reported(&ruff(&[("pyproject.toml", one_code)])), None);
}

#[test]
fn a_family_written_code_by_code_holds_the_family() {
    let by_code = "[tool.ruff.lint]\nselect = [\"A001\", \"A002\", \"A003\", \"A004\", \"A005\", \
                   \"A006\", \"ASYNC\", \"BLE\", \"SLOT\"]\n";
    assert_eq!(reported(&ruff(&[("pyproject.toml", by_code)])), None);
}

#[test]
fn with_explicit_preview_rules_a_rule_in_preview_needs_its_own_code() {
    let fragment = "[tool.ruff.lint]\npreview = true\nselect = [\"DOC102\"]\n";
    let family = "[tool.ruff.lint]\npreview = true\nexplicit-preview-rules = true\n\
                  select = [\"DOC\"]\n";
    let code = "[tool.ruff.lint]\npreview = true\nexplicit-preview-rules = true\n\
                select = [\"DOC102\"]\n";
    let implicit = "[tool.ruff.lint]\npreview = true\nselect = [\"DOC\"]\n";

    for (pyproject, held) in [(family, false), (code, true), (implicit, true)] {
        let found = findings(
            requirement("ruff", "pyproject.toml", fragment),
            &[("pyproject.toml", pyproject)],
        );
        assert_eq!(found.is_empty(), held, "{pyproject}");
    }
}

#[test]
fn a_family_that_is_neither_selected_nor_ignored_is_reported() {
    for pyproject in [
        "[tool.ruff.lint]\nselect = [\"E\", \"ASYNC\", \"BLE\", \"SLOT\"]\n",
        "[tool.ruff.lint]\nselect = [\"E\", \"ASYNC\", \"BLE\", \"SLOT\"]\nignore = [\"A005\"]\n",
        "[tool.ruff.lint]\nselect = [\"A001\", \"ASYNC\", \"BLE\", \"SLOT\"]\n",
        "[tool.ruff.lint]\nextend-select = [\"ASYNC\", \"BLE\", \"SLOT\"]\n",
    ] {
        let found = ruff(&[("pyproject.toml", pyproject)]);
        assert_eq!(reported(&found), Some("pyproject.toml"), "{pyproject}");
        assert_eq!(
            found[0].message,
            "pyproject.toml lacks the configuration of ruff that `the-rule` needs: \
             `botica explain the-rule` shows it"
        );
    }
}

#[test]
fn a_selector_takes_in_the_codes_of_its_linter_only() {
    let fragment = "[tool.ruff.lint]\nselect = [\"ASYNC\", \"PLR0124\"]\n";
    let only_a = "[tool.ruff.lint]\nselect = [\"A\", \"PL\"]\n";
    let found = findings(
        requirement("ruff", "pyproject.toml", fragment),
        &[("pyproject.toml", only_a)],
    );
    assert_eq!(reported(&found), Some("pyproject.toml"));

    let pylint = "[tool.ruff.lint]\nselect = [\"ASYNC\", \"PL\"]\n";
    let found = findings(
        requirement("ruff", "pyproject.toml", fragment),
        &[("pyproject.toml", pylint)],
    );
    assert_eq!(reported(&found), None);
}

#[test]
fn ruff_reads_dot_ruff_toml_then_ruff_toml_then_pyproject_toml() {
    let all_four = "[lint]\nselect = [\"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n";
    let none = "[lint]\nselect = [\"E\"]\n";
    let pyproject_all = "[tool.ruff.lint]\nselect = [\"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n";

    assert_eq!(reported(&ruff(&[("ruff.toml", all_four)])), None);
    assert_eq!(
        reported(&ruff(&[
            ("ruff.toml", none),
            ("pyproject.toml", pyproject_all)
        ])),
        Some("ruff.toml")
    );
    assert_eq!(
        reported(&ruff(&[(".ruff.toml", all_four), ("ruff.toml", none)])),
        None
    );
    assert_eq!(
        reported(&ruff(&[(".ruff.toml", none), ("ruff.toml", all_four)])),
        Some(".ruff.toml")
    );
}

#[test]
fn a_pyproject_toml_without_the_table_of_ruff_is_passed_over() {
    let project = "[project]\nname = \"x\"\n";
    let all_four = "[lint]\nselect = [\"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n";

    assert_eq!(
        reported(&ruff(&[
            ("pyproject.toml", project),
            ("ruff.toml", all_four)
        ])),
        None
    );
    assert_eq!(
        reported(&ruff(&[("pyproject.toml", project)])),
        Some("pyproject.toml")
    );

    let found = ruff(&[]);
    assert_eq!(
        found[0].message,
        "no file of settings of ruff exists (.ruff.toml, ruff.toml, pyproject.toml), and \
         `the-rule` needs one: `botica explain the-rule` shows what goes in it"
    );
}

#[test]
fn the_settings_that_a_file_extends_come_first_and_a_select_starts_again() {
    let base = "[lint]\nselect = [\"E\", \"ASYNC\", \"A\", \"BLE\", \"SLOT\"]\n";
    let extends = "[tool.ruff]\nextend = \"config/ruff-base.toml\"\n\n\
                   [tool.ruff.lint]\nextend-ignore = [\"SLOT\"]\n";
    assert_eq!(
        reported(&ruff(&[
            ("pyproject.toml", extends),
            ("config/ruff-base.toml", base)
        ])),
        None
    );

    let selects_again = "[tool.ruff]\nextend = \"config/ruff-base.toml\"\n\n\
                         [tool.ruff.lint]\nselect = [\"E\"]\n";
    assert_eq!(
        reported(&ruff(&[
            ("pyproject.toml", selects_again),
            ("config/ruff-base.toml", base)
        ])),
        Some("pyproject.toml")
    );

    let outside = "[tool.ruff]\nextend = \"../ruff.toml\"\n";
    assert_eq!(
        reported(&ruff(&[("pyproject.toml", outside)])),
        Some("pyproject.toml")
    );
}

#[test]
fn the_other_keys_of_a_ruff_fragment_are_compared_in_the_file_ruff_reads() {
    let fragment = "[tool.ruff.lint]\nselect = [\"D\"]\n\n[tool.ruff.lint.pydocstyle]\n\
                    convention = \"google\"\n";
    let google = "[lint]\nextend-select = [\"D\"]\n\n[lint.pydocstyle]\nconvention = \"google\"\n";
    let numpy = "[lint]\nextend-select = [\"D\"]\n\n[lint.pydocstyle]\nconvention = \"numpy\"\n";

    let holds = findings(
        requirement("ruff", "pyproject.toml", fragment),
        &[("ruff.toml", google)],
    );
    let differs = findings(
        requirement("ruff", "pyproject.toml", fragment),
        &[("ruff.toml", numpy)],
    );

    assert_eq!(reported(&holds), None);
    assert_eq!(reported(&differs), None);
    assert_eq!(
        weaker(
            requirement("ruff", "pyproject.toml", fragment),
            &[("ruff.toml", numpy)]
        ),
        [(
            "lint.pydocstyle.convention".to_owned(),
            "\"google\"".to_owned(),
            "\"numpy\"".to_owned()
        )]
    );
}

#[test]
fn pyrefly_holds_a_preset_with_one_that_checks_at_least_as_much() {
    let fragment = "[tool.pyrefly]\npreset = \"strict\"\n";
    for (pyproject, reported_it, weakened_it) in [
        ("[tool.pyrefly]\npreset = \"strict\"\n", false, false),
        ("[tool.pyrefly]\npreset = \"all\"\n", false, false),
        ("[tool.pyrefly]\npreset = \"basic\"\n", false, true),
        ("[tool.pyrefly]\npreset = \"default\"\n", false, true),
        (
            "[tool.pyrefly]\nproject-includes = [\"src\"]\n",
            true,
            false,
        ),
    ] {
        let files = [("pyproject.toml", pyproject)];
        let found = findings(requirement("pyrefly", "pyproject.toml", fragment), &files);
        let weak = weaker(requirement("pyrefly", "pyproject.toml", fragment), &files);
        assert_eq!(!found.is_empty(), reported_it, "{pyproject}");
        assert_eq!(!weak.is_empty(), weakened_it, "{pyproject}");
    }

    let in_pyrefly_toml = findings(
        requirement("pyrefly", "pyproject.toml", fragment),
        &[
            ("pyrefly.toml", "preset = \"all\"\n"),
            ("pyproject.toml", "[tool.pyrefly]\npreset = \"basic\"\n"),
        ],
    );
    assert_eq!(reported(&in_pyrefly_toml), None);
}

#[test]
fn a_kind_of_error_of_pyrefly_holds_when_the_project_sets_it_as_high_or_the_preset_is_all() {
    let fragment =
        "[tool.pyrefly]\npreset = \"strict\"\n\n[tool.pyrefly.errors]\nexplicit-any = true\n";
    for (errors, reported_it, weakened_it) in [
        ("[tool.pyrefly.errors]\nexplicit-any = true\n", false, false),
        (
            "[tool.pyrefly.errors]\nexplicit-any = \"error\"\n",
            false,
            false,
        ),
        ("[tool.pyrefly.errors]\nexplicit-any = false\n", false, true),
        (
            "[tool.pyrefly.errors]\nexplicit-any = \"warn\"\n",
            false,
            true,
        ),
        ("", true, false),
    ] {
        let pyproject = format!("[tool.pyrefly]\npreset = \"strict\"\n\n{errors}");
        let files = [("pyproject.toml", pyproject.as_str())];
        let found = findings(requirement("pyrefly", "pyproject.toml", fragment), &files);
        let weak = weaker(requirement("pyrefly", "pyproject.toml", fragment), &files);
        assert_eq!(!found.is_empty(), reported_it, "{pyproject}");
        assert_eq!(!weak.is_empty(), weakened_it, "{pyproject}");
    }

    let all = findings(
        requirement("pyrefly", "pyproject.toml", fragment),
        &[("pyproject.toml", "[tool.pyrefly]\npreset = \"all\"\n")],
    );
    assert_eq!(reported(&all), None);
}

#[test]
fn pytest_reads_the_first_file_that_matches_and_compares_text_with_typed_values() {
    let fragment = "[tool.pytest]\nxfail_strict = true\naddopts = [\"--strict-markers\"]\n";
    let ini = "[pytest]\nxfail_strict = True\naddopts = -ra --strict-markers\n";
    let ini_options =
        "[tool.pytest.ini_options]\nxfail_strict = \"true\"\naddopts = \"-ra --strict-markers\"\n";
    let native = "[tool.pytest]\nxfail_strict = true\naddopts = [\"-ra\", \"--strict-markers\"]\n";

    for files in [
        vec![("pytest.ini", ini)],
        vec![("pyproject.toml", ini_options)],
        vec![("pyproject.toml", native)],
        vec![(
            "setup.cfg",
            "[tool:pytest]\nxfail_strict = true\naddopts =\n    --strict-markers\n",
        )],
    ] {
        let found = findings(requirement("pytest", "pyproject.toml", fragment), &files);
        assert_eq!(reported(&found), None, "{files:?}");
    }

    let empty_pytest_toml = findings(
        requirement("pytest", "pyproject.toml", fragment),
        &[("pytest.toml", ""), ("pyproject.toml", native)],
    );
    assert_eq!(reported(&empty_pytest_toml), Some("pytest.toml"));
}

#[test]
fn a_cooldown_holds_with_a_span_at_least_as_long_written_any_way_the_tool_reads() {
    let cases = [
        (
            "uv",
            "pyproject.toml",
            "[tool.uv]\nexclude-newer = \"7 days\"\n",
            vec![
                ("[tool.uv]\nexclude-newer = \"1 week\"\n", true),
                ("[tool.uv]\nexclude-newer = \"P7D\"\n", true),
                ("[tool.uv]\nexclude-newer = \"14 days\"\n", true),
                ("[tool.uv]\nexclude-newer = \"3 days\"\n", false),
                ("[tool.uv]\nexclude-newer = false\n", false),
                (
                    "[tool.uv]\nexclude-newer = \"2026-10-01T00:00:00Z\"\n",
                    false,
                ),
            ],
        ),
        (
            "pnpm",
            "pnpm-workspace.yaml",
            "minimumReleaseAge: 10080\n",
            vec![
                ("packages:\n  - app\nminimumReleaseAge: 10080\n", true),
                ("minimumReleaseAge: 20160\n", true),
                ("minimumReleaseAge: 1440\n", false),
                ("minimumReleaseAge: 0\n", false),
            ],
        ),
        (
            "yarn",
            ".yarnrc.yml",
            "npmMinimalAgeGate: \"7d\"\n",
            vec![
                ("npmMinimalAgeGate: 1w\n", true),
                ("npmMinimalAgeGate: 10080\n", true),
                ("npmMinimalAgeGate: 3d\n", false),
            ],
        ),
        (
            "npm",
            ".npmrc",
            "min-release-age=7\n",
            vec![
                (
                    "registry=https://registry.npmjs.org/\nmin-release-age=7\n",
                    true,
                ),
                ("min-release-age = 14\n", true),
                ("min-release-age=3\n", false),
            ],
        ),
        (
            "cargo",
            ".cargo/config.toml",
            "[registry]\nglobal-min-publish-age = \"7 days\"\n",
            vec![
                ("[registry]\nglobal-min-publish-age = \"1 weeks\"\n", true),
                ("[registry]\nglobal-min-publish-age = \"0\"\n", false),
            ],
        ),
    ];
    for (tool, target, fragment, files) in cases {
        for (text, held) in files {
            let files = [(target, text)];
            let found = findings(requirement(tool, target, fragment), &files);
            let weak = weaker(requirement(tool, target, fragment), &files);
            assert_eq!(reported(&found), None, "{tool}: {text}");
            assert_eq!(weak.is_empty(), held, "{tool}: {text}");
        }
        let without_the_key = match tool {
            "uv" => "[tool.uv]\nindex-url = \"https://example.org/simple\"\n",
            "npm" => "registry=https://registry.npmjs.org/\n",
            "cargo" => "[build]\njobs = 2\n",
            _ => "nodeLinker: node-modules\n",
        };
        let lacks = findings(
            requirement(tool, target, fragment),
            &[(target, without_the_key)],
        );
        assert_eq!(reported(&lacks), Some(target), "{tool}");
    }
}

#[test]
fn a_weakening_names_the_rule_the_tool_the_file_the_key_and_both_values() {
    let found = weakenings(
        requirement(
            "uv",
            "pyproject.toml",
            "[tool.uv]\nexclude-newer = \"7 days\"\n",
        ),
        &[
            ("uv.toml", "exclude-newer = \"3 days\"\n"),
            ("pyproject.toml", "[tool.uv]\nexclude-newer = \"7 days\"\n"),
        ],
    );

    assert_eq!(
        found,
        [Weakening {
            rule: "the-rule".to_owned(),
            tool: "uv".to_owned(),
            file: "uv.toml".to_owned(),
            key: "exclude-newer".to_owned(),
            wanted: "\"7 days\"".to_owned(),
            found: "\"3 days\"".to_owned(),
        }]
    );
}

#[test]
fn uv_reads_uv_toml_before_pyproject_toml_and_cargo_its_old_file_first() {
    let fragment = "[tool.uv]\nexclude-newer = \"7 days\"\n";
    let found = findings(
        requirement("uv", "pyproject.toml", fragment),
        &[
            ("uv.toml", "index-url = \"https://example.org/simple\"\n"),
            ("pyproject.toml", "[tool.uv]\nexclude-newer = \"7 days\"\n"),
        ],
    );
    assert_eq!(reported(&found), Some("uv.toml"));

    let cargo = "[registry]\nglobal-min-publish-age = \"7 days\"\n";
    let found = findings(
        requirement("cargo", ".cargo/config.toml", cargo),
        &[
            (".cargo/config", "[build]\njobs = 2\n"),
            (".cargo/config.toml", cargo),
        ],
    );
    assert_eq!(reported(&found), Some(".cargo/config"));
}

#[test]
fn a_file_that_is_not_in_its_format_is_reported_with_the_format() {
    let found = findings(
        requirement("pnpm", "pnpm-workspace.yaml", "minimumReleaseAge: 10080\n"),
        &[("pnpm-workspace.yaml", "packages: [app\n")],
    );

    assert_eq!(reported(&found), Some("pnpm-workspace.yaml"));
    assert!(
        found[0].message.starts_with(
            "pnpm-workspace.yaml is not YAML, so botica cannot find the configuration of pnpm \
             that `the-rule` needs: "
        ),
        "{}",
        found[0].message
    );
}

#[test]
fn a_fragment_for_a_file_that_is_not_a_place_of_its_tool_is_read_from_that_file() {
    let fragment = "[[repos]]\n[[repos.hooks]]\nid = \"ruff-check\"\n";
    let prek = "[[repos]]\nrepo = \"local\"\n\n[[repos.hooks]]\nid = \"ruff-check\"\n";

    let found = findings(
        requirement("ruff", "prek.toml", fragment),
        &[("prek.toml", prek)],
    );

    assert_eq!(reported(&found), None);
}

#[test]
fn a_project_uses_a_tool_when_a_glob_of_applies_when_matches_a_file() {
    let globs = |globs: &[&str]| {
        globs
            .iter()
            .map(|glob| (*glob).to_owned())
            .collect::<Vec<_>>()
    };
    let files = |files: &[&str]| {
        files
            .iter()
            .map(|file| (*file).to_owned())
            .collect::<Vec<_>>()
    };
    let none: Vec<String> = Vec::new();

    let lockfile = globs(&["pnpm-lock.yaml"]);
    assert!(in_use(
        &none,
        &lockfile,
        &files(&["pnpm-lock.yaml", "src/a.ts"])
    ));
    assert!(!in_use(&none, &lockfile, &files(&["app/pnpm-lock.yaml"])));

    let docker = globs(&["**/Dockerfile", "**/*.Dockerfile", "**/Containerfile"]);
    for file in [
        "Dockerfile",
        "deploy/api.Dockerfile",
        ".devcontainer/Containerfile",
    ] {
        assert!(in_use(&none, &docker, &files(&[file])), "{file}");
    }
    assert!(!in_use(&none, &docker, &files(&["docs/Dockerfile.md"])));

    let terraform = globs(&["**/*.tf"]);
    assert!(in_use(&none, &terraform, &files(&["infra/main.tf"])));
    assert!(!in_use(&none, &terraform, &files(&["infra/main.tfvars"])));
}

#[test]
fn without_applies_when_a_file_of_one_of_its_languages_decides() {
    let strings = |items: &[&str]| {
        items
            .iter()
            .map(|item| (*item).to_owned())
            .collect::<Vec<_>>()
    };
    let none: Vec<String> = Vec::new();

    assert!(in_use(
        &strings(&["python"]),
        &none,
        &strings(&["src/a.py"])
    ));
    assert!(!in_use(
        &strings(&["python"]),
        &none,
        &strings(&["main.go"])
    ));
    assert!(in_use(
        &strings(&["tsx"]),
        &none,
        &strings(&["src/App.tsx"])
    ));
    assert!(!in_use(&strings(&["tsx"]), &none, &strings(&["src/a.ts"])));
    assert!(in_use(&strings(&["*"]), &none, &none));
    assert!(in_use(&strings(&["dockerfile"]), &none, &none));
}
