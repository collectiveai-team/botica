//! `botica rules new --catalog .botica/rules/<name>` writes a rule into a catalog of a project.

mod common;

use std::path::{Path, PathBuf};
use std::process::Output;

use common::{command, project_in, stderr, stdout, write};

const CATALOG: &str = ".botica/rules/acme";

fn project(name: &str) -> PathBuf {
    let dir = project_in("rules-new-project", name, Some(""));
    write(
        &dir,
        &format!("{CATALOG}/catalog.toml"),
        "code = \"ACME\"\n",
    );
    write(
        &dir,
        &format!("{CATALOG}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    dir
}

fn new_rule(dir: &Path, args: &[&str]) -> Output {
    let mut all = vec!["rules", "new"];
    all.extend(args);
    all.extend(["--catalog", CATALOG]);
    command(dir, &all)
}

fn read(dir: &Path, relative: &str) -> String {
    let path = dir.join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// What `botica rules --all` lists: the project must load with the new rule.
fn listed(dir: &Path) -> String {
    let output = command(dir, &["rules", "--all"]);
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output)
}

#[test]
fn a_python_extension_gets_the_id_and_the_code_of_its_catalog_and_a_script_that_loads() {
    let dir = project("python");

    let output = new_rule(
        &dir,
        &[
            "structure/no-blobs",
            "--kind",
            "extension",
            "--runtime",
            "python",
            "--language",
            "python",
        ],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let folder = format!("{CATALOG}/structure/no-blobs");
    assert_eq!(
        read(&dir, &format!("{folder}/rule.toml")),
        "id = \"acme/no-blobs\"\n\
         code = \"ACMESTRUC001\"\n\
         summary = \"TODO: what the rule asks, in one line\"\n\
         status = \"preview\"\n\
         scope = \"file\"\n\
         needs = \"syntax\"\n\
         run-on = [\"cli\"]\n\
         default-enabled = false\n\
         default-severity = \"warning\"\n\
         \n\
         [[implementations]]\n\
         kind = \"extension\"\n\
         runtime = \"python\"\n\
         languages = [\"python\"]\n\
         file = \"no-blobs.py\"\n"
    );
    let script = read(&dir, &format!("{folder}/no-blobs.py"));
    assert!(
        script.contains(&format!(
            "dependencies = [\"botica[sdk]=={}\"]",
            env!("CARGO_PKG_VERSION")
        )),
        "{script}"
    );
    assert!(
        script.contains("@rule(\"acme/no-blobs\", [\"python\"])\ndef check(path, text, options):"),
        "the check of the SDK gets the options of the rule:\n{script}"
    );
    assert!(dir
        .join(&folder)
        .join("fixtures/valid/example.py")
        .is_file());
    assert!(dir
        .join(&folder)
        .join("fixtures/invalid/example.py")
        .is_file());
    let text = stdout(&output);
    assert!(
        text.contains("structure/no-blobs") && text.contains("ACMESTRUC001"),
        "{text}"
    );
    assert!(listed(&dir).contains("ACMESTRUC001"), "{}", listed(&dir));
}

#[test]
fn the_next_rule_of_the_family_takes_the_next_code() {
    let dir = project("next-code");
    for slug in ["structure/first", "structure/second"] {
        let output = new_rule(
            &dir,
            &[
                slug,
                "--kind",
                "extension",
                "--runtime",
                "node",
                "--language",
                "javascript",
            ],
        );
        assert!(output.status.success(), "{}", stderr(&output));
    }

    let rules = read(&dir, &format!("{CATALOG}/structure/second/rule.toml"));
    assert!(rules.contains("code = \"ACMESTRUC002\""), "{rules}");
    assert!(rules.contains("runtime = \"node\""), "{rules}");
    assert!(rules.contains("file = \"second.mjs\""), "{rules}");
    let script = read(&dir, &format!("{CATALOG}/structure/second/second.mjs"));
    assert!(script.contains("const RULE = \"acme/second\";"), "{script}");
    assert!(script.contains("languages: [\"javascript\"]"), "{script}");
    assert!(dir
        .join(CATALOG)
        .join("structure/second/second.mjs")
        .is_file());
}

#[test]
fn a_command_extension_names_a_script_by_its_path_from_the_root_of_the_project() {
    let dir = project("command");

    let output = new_rule(
        &dir,
        &[
            "structure/by-sh",
            "--kind",
            "extension",
            "--runtime",
            "command",
            "--language",
            "python",
        ],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let rule = read(&dir, &format!("{CATALOG}/structure/by-sh/rule.toml"));
    assert!(
        rule.contains("command = [\"sh\", \".botica/rules/acme/structure/by-sh/check.sh\"]"),
        "{rule}"
    );
    assert!(!rule.contains("file ="), "{rule}");
    assert!(dir.join(CATALOG).join("structure/by-sh/check.sh").is_file());
    assert!(listed(&dir).contains("acme/by-sh"), "{}", listed(&dir));
}

#[test]
fn an_ast_grep_rule_of_a_project_declares_the_full_id_in_its_yaml() {
    let dir = project("ast-grep");

    let output = new_rule(
        &dir,
        &[
            "structure/no-foo",
            "--kind",
            "ast-grep",
            "--language",
            "python",
            "--standard",
            "ACME-1",
        ],
    );

    assert!(output.status.success(), "{}", stderr(&output));
    let yaml = read(&dir, &format!("{CATALOG}/structure/no-foo/python.yml"));
    assert!(yaml.starts_with("id: acme/no-foo\n"), "{yaml}");
    assert!(yaml.contains("message: \"TODO"), "{yaml}");
    assert!(listed(&dir).contains("ACMESTRUC001"), "{}", listed(&dir));
}

#[test]
fn only_the_families_of_the_catalog_are_offered() {
    let dir = project("family");

    let output = new_rule(
        &dir,
        &[
            "architecture/x",
            "--kind",
            "ast-grep",
            "--language",
            "python",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    let message = stderr(&output);
    assert!(message.contains("`architecture`"), "{message}");
    assert!(message.ends_with("structure\n"), "{message}");
    assert!(!dir.join(CATALOG).join("architecture").exists());
}

#[test]
fn an_extension_needs_a_runtime_and_a_runtime_needs_an_extension() {
    let dir = project("runtime-flag");

    let without = new_rule(
        &dir,
        &["structure/a", "--kind", "extension", "--language", "python"],
    );
    let stray = new_rule(
        &dir,
        &[
            "structure/b",
            "--kind",
            "ast-grep",
            "--runtime",
            "python",
            "--language",
            "python",
        ],
    );

    assert_eq!(without.status.code(), Some(2));
    assert!(
        stderr(&without).contains("--runtime"),
        "{}",
        stderr(&without)
    );
    assert_eq!(stray.status.code(), Some(2));
    assert!(stderr(&stray).contains("--runtime"), "{}", stderr(&stray));
    assert!(!dir.join(CATALOG).join("structure/a").exists());
    assert!(!dir.join(CATALOG).join("structure/b").exists());
}

#[test]
fn a_catalog_that_does_not_load_is_left_without_the_new_rule() {
    let dir = project("no-write-on-error");
    write(
        &dir,
        &format!("{CATALOG}/structure/taken/rule.toml"),
        "this is not a rule\n",
    );

    let output = new_rule(
        &dir,
        &[
            "structure/new",
            "--kind",
            "extension",
            "--runtime",
            "python",
            "--language",
            "python",
        ],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(!dir.join(CATALOG).join("structure/new").exists());
}
