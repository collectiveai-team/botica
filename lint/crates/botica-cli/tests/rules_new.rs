use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use botica_catalog::{read_files, Catalog};

fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create a directory");
    for entry in fs::read_dir(from).expect("read the catalog").flatten() {
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy a file");
        }
    }
}

/// A working directory with a copy of the catalog of this checkout under `cat/`.
fn workdir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("rules-new")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../rules"),
        &dir.join("cat"),
    );
    dir
}

fn new_rule(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_botica"))
        .args(["rules", "new"])
        .args(args)
        .args(["--catalog", "cat"])
        .current_dir(dir)
        .output()
        .expect("run botica")
}

fn succeeded(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn reload(dir: &Path) -> Catalog {
    let files = read_files(&dir.join("cat")).expect("read the catalog");
    Catalog::from_files(&files).unwrap_or_else(|error| panic!("the catalog loads:\n{error}"))
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

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
         code = \"BTSTRUC006\"\n\
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
         message: \"CES-63 (no-foo): TODO: the rule and its fix\"\n\
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
    let rule = catalog.resolve("BTSTRUC006").expect("the new rule");
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
        stdout.contains("BTSTRUC006") && stdout.contains("structure/no-foo"),
        "{stdout}"
    );
}

#[test]
fn an_extension_rule_names_its_runtime_and_has_no_yaml() {
    let dir = workdir("extension");

    let output = new_rule(
        &dir,
        &[
            "logging/log-foo",
            "--kind",
            "extension",
            "--language",
            "typescript",
            "--runtime",
            "node",
        ],
    );

    succeeded(&output);
    let folder = dir.join("cat/logging/log-foo");
    let toml = read(&folder.join("rule.toml"));
    assert!(
        toml.starts_with("id = \"log-foo\"\ncode = \"BTLOG003\"\nsummary = "),
        "{toml}"
    );
    assert!(toml.contains("needs = \"bytes\"\n"), "{toml}");
    assert!(
        toml.ends_with(
            "[[implementations]]\nkind = \"extension\"\nruntime = \"node\"\n\
             languages = [\"typescript\"]\n"
        ),
        "{toml}"
    );
    assert!(!folder.join("typescript.yml").exists());
    assert_eq!(
        read(&folder.join("fixtures/valid/example.ts")),
        "// TODO: code that follows the rule\n"
    );
    assert!(folder.join("fixtures/invalid/example.ts").is_file());
    assert_eq!(
        reload(&dir)
            .resolve("BTLOG003")
            .map(|rule| rule.id.as_str()),
        Some("log-foo")
    );
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
        assert!(message.contains("architecture, asr,"), "{message}");
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
fn an_extension_needs_a_runtime_and_only_an_extension_takes_one() {
    let dir = workdir("runtime");

    let no_runtime = new_rule(
        &dir,
        &[
            "structure/no-foo",
            "--kind",
            "extension",
            "--language",
            "python",
        ],
    );
    assert_eq!(no_runtime.status.code(), Some(2));
    assert!(
        stderr(&no_runtime).contains("--runtime"),
        "{}",
        stderr(&no_runtime)
    );

    let runtime = new_rule(
        &dir,
        &[
            "structure/no-foo",
            "--kind",
            "ast-grep",
            "--language",
            "python",
            "--runtime",
            "python",
        ],
    );
    assert_eq!(runtime.status.code(), Some(2));
    assert!(
        stderr(&runtime).contains("--runtime applies to an extension only"),
        "{}",
        stderr(&runtime)
    );
    assert!(!dir.join("cat/structure/no-foo").exists());
}

#[test]
fn the_message_of_a_rule_without_a_standard_starts_with_its_slug() {
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
        yaml.contains("message: \"no-foo: TODO: the rule and its fix\"\n"),
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
         code = \"BTTEST001\"\n\
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
         instructions = \"TODO: the question the judge answers about the code\"\n"
    );
    assert!(!folder.join("python.yml").exists());
    assert_eq!(
        read(&folder.join("fixtures/expected.json")),
        "{\n  \"verdicts\": [\n    { \"file\": \"valid/example.py\", \"band\": \"clear\" },\n    \
         { \"file\": \"invalid/example.py\", \"band\": \"act\" }\n  ]\n}\n"
    );
    let catalog = reload(&dir);
    let rule = catalog.resolve("BTTEST001").expect("the new rule");
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
