//! `botica rules new --kind external`: the folder of a rule that another tool enforces.

mod common;

use common::new_rule::{new_rule, read, reload, stderr_of, succeeded, workdir};

#[test]
fn an_external_rule_names_its_tool_and_gets_the_fragment_of_its_configuration() {
    let dir = workdir("external");

    let output = new_rule(
        &dir,
        &[
            "structure/dep-hygiene",
            "--kind",
            "external",
            "--language",
            "python",
            "--standard",
            "CES-109",
            "--tool",
            "deptry",
            "--docs",
            "https://deptry.com/",
            "--config",
            "prek.toml",
        ],
    );

    succeeded(&output);
    let folder = dir.join("cat/structure/dep-hygiene");
    assert_eq!(
        read(&folder.join("rule.toml")),
        "id = \"dep-hygiene\"\n\
         code = \"BTSTRUC007\"\n\
         standard = \"CES-109\"\n\
         summary = \"TODO: what the rule asks, in one line\"\n\
         status = \"preview\"\n\
         default-enabled = false\n\
         default-severity = \"warning\"\n\
         \n\
         [[implementations]]\n\
         kind = \"external\"\n\
         languages = [\"python\"]\n\
         tool = \"deptry\"\n\
         docs = \"https://deptry.com/\"\n\
         codes = []\n\
         config = \"config/prek.toml\"\n"
    );
    assert_eq!(
        read(&folder.join("config/prek.toml")),
        "# TODO: the configuration of deptry in prek.toml that enforces the rule\n"
    );
    assert!(!folder.join("fixtures").exists());

    let catalog = reload(&dir);
    let rule = catalog.resolve("BTSTRUC007").expect("the new rule");
    assert!(rule.is_external_only());
    assert_eq!(rule.implementations[0].target(), Some("prek.toml"));
    let titles: Vec<String> = rule
        .sections()
        .into_iter()
        .map(|section| section.title)
        .collect();
    assert_eq!(
        titles,
        [
            "Directive",
            "Why",
            "Tool",
            "Configuration",
            "Do",
            "Don't",
            "Suppression"
        ]
    );
    assert!(rule
        .sections()
        .iter()
        .all(|section| section.text.starts_with("TODO: ")));
}

#[test]
fn an_external_rule_without_config_has_no_fragment() {
    let dir = workdir("external-no-config");

    let output = new_rule(
        &dir,
        &[
            "structure/dep-hygiene",
            "--kind",
            "external",
            "--language",
            "python",
            "--tool",
            "deptry",
            "--docs",
            "https://deptry.com/",
        ],
    );

    succeeded(&output);
    let folder = dir.join("cat/structure/dep-hygiene");
    let toml = read(&folder.join("rule.toml"));
    assert!(!toml.contains("config"), "{toml}");
    assert!(!folder.join("config").exists());
    assert!(reload(&dir).resolve("dep-hygiene").is_some());
}

#[test]
fn an_external_rule_needs_its_tool_and_its_docs_and_only_it_takes_them() {
    let dir = workdir("external-flags");
    let external = [
        "structure/dep-hygiene",
        "--kind",
        "external",
        "--language",
        "python",
    ];

    for (missing, args) in [
        ("--tool", vec!["--docs", "https://deptry.com/"]),
        ("--docs", vec!["--tool", "deptry"]),
    ] {
        let output = new_rule(&dir, &[&external[..], &args].concat());
        assert_eq!(output.status.code(), Some(2), "{missing}");
        assert!(
            stderr_of(&output).contains(missing),
            "{}",
            stderr_of(&output)
        );
    }

    for flag in [
        ["--tool", "deptry"],
        ["--docs", "https://deptry.com/"],
        ["--config", "prek.toml"],
    ] {
        let output = new_rule(
            &dir,
            &[
                &[
                    "structure/no-foo",
                    "--kind",
                    "ast-grep",
                    "--language",
                    "python",
                ][..],
                &flag,
            ]
            .concat(),
        );
        assert_eq!(output.status.code(), Some(2), "{flag:?}");
        assert!(
            stderr_of(&output)
                .contains("--tool, --docs and --config apply to an external rule only"),
            "{}",
            stderr_of(&output)
        );
    }
    assert!(!dir.join("cat/structure/no-foo").exists());
}

#[test]
fn the_configuration_of_a_new_rule_is_a_toml_file_of_the_root_of_the_project() {
    let dir = workdir("external-bad-config");

    let output = new_rule(
        &dir,
        &[
            "structure/dep-hygiene",
            "--kind",
            "external",
            "--language",
            "python",
            "--tool",
            "deptry",
            "--docs",
            "https://deptry.com/",
            "--config",
            ".pre-commit-config.yaml",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr_of(&output)
            .contains("config `config/.pre-commit-config.yaml` must be `config/<file>.toml`"),
        "{}",
        stderr_of(&output)
    );
    assert!(!dir.join("cat/structure/dep-hygiene").exists());
}
