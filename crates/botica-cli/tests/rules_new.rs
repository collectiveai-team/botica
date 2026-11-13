mod common;

use common::new_rule::{new_rule, read, reload, stderr_of as stderr, succeeded, workdir};

#[test]
fn an_ast_grep_rule_gets_the_next_code_of_its_family_and_a_folder_that_loads() {
    let dir = workdir("ast-grep");

    let output = new_rule(
        &dir,
        &[
            "structure/no-foo",
            "--kind",
            "ast-grep",
            "--language",
            "python",
            "--standard",
            "CES-63",
        ],
    );

    succeeded(&output);
    let folder = dir.join("cat/structure/no-foo");
    assert_eq!(
        read(&folder.join("rule.toml")),
        "id = \"no-foo\"\n\
         code = \"BTSTRUC008\"\n\
         standard = \"CES-63\"\n\
         summary = \"TODO: what the rule asks, in one line\"\n\
         status = \"preview\"\n\
         scope = \"file\"\n\
         needs = \"syntax\"\n\
         run-on = [\"cli\", \"save\", \"type\"]\n\
         default-enabled = false\n\
         default-severity = \"warning\"\n\
         \n\
         [[implementations]]\n\
         kind = \"ast-grep\"\n\
         languages = [\"python\"]\n\
         file = \"python.yml\"\n"
    );
    assert_eq!(
        read(&folder.join("python.yml")),
        "id: no-foo\n\
         language: python\n\
         severity: warning\n\
         message: \"TODO: the rule and its fix\"\n\
         rule:\n  pattern: TODO\n"
    );
    assert_eq!(
        read(&folder.join("fixtures/expected.json")),
        "{\n  \"diagnostics\": []\n}\n"
    );
    assert_eq!(
        read(&folder.join("fixtures/valid/example.py")),
        "# TODO: code that follows the rule\n"
    );
    assert_eq!(
        read(&folder.join("fixtures/invalid/example.py")),
        "# TODO: code that breaks the rule, with its findings in ../expected.json\n"
    );

    let catalog = reload(&dir);
    let rule = catalog.resolve("BTSTRUC008").expect("the new rule");
    assert_eq!(rule.id, "no-foo");
    let titles: Vec<String> = rule
        .sections()
        .into_iter()
        .map(|section| section.title)
        .collect();
    assert_eq!(titles, ["Directive", "Why", "Do", "Don't", "Suppression"]);
    assert!(rule
        .sections()
        .iter()
        .all(|section| section.text.starts_with("TODO: ")));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("BTSTRUC008") && stdout.contains("structure/no-foo"),
        "{stdout}"
    );
}

#[test]
fn an_extension_is_not_a_kind_of_the_built_in_catalog() {
    let dir = workdir("extension");

    let output = new_rule(
        &dir,
        &[
            "logging/log-foo",
            "--kind",
            "extension",
            "--runtime",
            "python",
            "--language",
            "python",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("an extension is a program of a project"),
        "{}",
        stderr(&output)
    );
    assert!(!dir.join("cat/logging/log-foo").exists());
}

#[test]
fn an_existing_folder_is_left_as_it_is() {
    let dir = workdir("exists");
    let readme = dir.join("cat/structure/no-utils/README.md");
    let before = read(&readme);

    let output = new_rule(
        &dir,
        &[
            "structure/no-utils",
            "--kind",
            "ast-grep",
            "--language",
            "python",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("structure/no-utils"),
        "{}",
        stderr(&output)
    );
    assert_eq!(read(&readme), before);
    assert!(!dir.join("cat/structure/no-utils/python.yml").exists());
}

#[test]
fn a_family_that_the_catalog_does_not_have_is_rejected_with_the_ones_it_has() {
    let dir = workdir("family");

    for family in ["nope", "botica"] {
        let output = new_rule(
            &dir,
            &[
                &format!("{family}/no-foo"),
                "--kind",
                "ast-grep",
                "--language",
                "python",
            ],
        );

        assert_eq!(output.status.code(), Some(2));
        let message = stderr(&output);
        assert!(message.contains(&format!("`{family}`")), "{message}");
        assert!(
            message.contains("architecture, async, cli, config,"),
            "{message}"
        );
        assert!(!dir.join(format!("cat/{family}/no-foo")).exists());
    }
}

#[test]
fn a_native_rule_is_rejected_because_its_code_lives_in_botica_core() {
    let dir = workdir("native");

    let output = new_rule(
        &dir,
        &[
            "structure/no-foo",
            "--kind",
            "native",
            "--language",
            "python",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("botica-core"),
        "{}",
        stderr(&output)
    );
    assert!(!dir.join("cat/structure/no-foo").exists());
}

#[test]
fn a_rule_the_catalog_would_reject_is_not_written() {
    let dir = workdir("invalid");

    for (slug, standard) in [("No_Foo", "CES-63"), ("no-foo", "CES63")] {
        let output = new_rule(
            &dir,
            &[
                &format!("structure/{slug}"),
                "--kind",
                "ast-grep",
                "--language",
                "python",
                "--standard",
                standard,
            ],
        );

        assert_eq!(output.status.code(), Some(2), "{slug} {standard}");
        assert!(!dir.join(format!("cat/structure/{slug}")).exists());
    }
}

#[test]
fn a_rule_without_a_standard_has_no_standard_field() {
    let dir = workdir("no-standard");

    let output = new_rule(
        &dir,
        &[
            "structure/no-foo",
            "--kind",
            "ast-grep",
            "--language",
            "python",
        ],
    );

    succeeded(&output);
    let yaml = read(&dir.join("cat/structure/no-foo/python.yml"));
    assert!(
        yaml.contains("message: \"TODO: the rule and its fix\"\n"),
        "{yaml}"
    );
    let toml = read(&dir.join("cat/structure/no-foo/rule.toml"));
    assert!(!toml.contains("standard"), "{toml}");
}

#[test]
fn a_judgment_rule_has_the_fields_of_a_judge_and_the_readme_of_its_kind() {
    let dir = workdir("judgment");

    let output = new_rule(
        &dir,
        &[
            "testing/test-foo",
            "--kind",
            "judgment",
            "--language",
            "python",
            "--standard",
            "CES-65",
        ],
    );

    succeeded(&output);
    let folder = dir.join("cat/testing/test-foo");
    assert_eq!(
        read(&folder.join("rule.toml")),
        "id = \"test-foo\"\n\
         code = \"BTTEST013\"\n\
         standard = \"CES-65\"\n\
         summary = \"TODO: what the rule asks, in one line\"\n\
         status = \"preview\"\n\
         default-enabled = false\n\
         default-severity = \"warning\"\n\
         \n\
         [[implementations]]\n\
         kind = \"judgment\"\n\
         languages = [\"python\"]\n\
         when = \"edit\"\n\
         why-not-deterministic = \"TODO: why no check of the code or of the configuration of a \
         repository decides it\"\n\
         \n\
         [implementations.question]\n\
         type = \"boolean\"\n\
         instructions = \"TODO: does the code break the rule? A yes (true) is a violation\"\n"
    );
    assert!(!folder.join("python.yml").exists());
    assert_eq!(
        read(&folder.join("fixtures/expected.json")),
        "{\n  \"verdicts\": [\n    { \"file\": \"valid/example.py\", \"band\": \"clear\" },\n    \
         { \"file\": \"invalid/example.py\", \"band\": \"act\" }\n  ]\n}\n"
    );
    let catalog = reload(&dir);
    let rule = catalog.resolve("BTTEST013").expect("the new rule");
    assert!(rule.is_judgment());
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
            "Flag",
            "Do not flag",
            "Confirm",
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
