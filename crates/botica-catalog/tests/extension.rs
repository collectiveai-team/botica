//! An extension rule names the code that runs it: `file` for `python` and `node`, `command` for
//! `command`. The catalog loads that code as it loads the YAML of an ast-grep rule.

mod common;

use botica_catalog::{Catalog, SourceFile};
use common::project::{acme, built_in, project_rule, rejected};
use common::{family, file};

fn implementation(runtime: &str, fields: &str) -> String {
    format!(
        "[[implementations]]\nkind = \"extension\"\nruntime = \"{runtime}\"\n\
         languages = [\"python\"]\n{fields}"
    )
}

/// The project catalog `acme` with one more rule, `acme/by-code`, and the files given.
fn with_rule(implementation: &str, extra: Vec<SourceFile>) -> Vec<SourceFile> {
    let mut files = acme();
    files.extend(project_rule(
        "acme",
        "structure",
        "by-code",
        "ACMESTRUC002",
        "",
        implementation,
    ));
    files.extend(extra);
    files
}

fn script(name: &str, text: &str) -> SourceFile {
    file(&format!("acme/structure/by-code/{name}"), text)
}

fn loaded(files: &[SourceFile]) -> Catalog {
    built_in()
        .with_projects(files)
        .unwrap_or_else(|error| panic!("{error}"))
}

#[test]
fn a_python_extension_names_its_script_and_the_catalog_carries_its_text() {
    let files = with_rule(
        &implementation("python", "file = \"by-code.py\"\n"),
        vec![script("by-code.py", "print(1)\n")],
    );

    let rule = loaded(&files)
        .resolve("acme/by-code")
        .cloned()
        .expect("rule");

    let implementation = &rule.implementations[0];
    assert_eq!(implementation.file.as_deref(), Some("by-code.py"));
    assert_eq!(implementation.source.as_deref(), Some("print(1)\n"));
    assert_eq!(implementation.command, None);
}

#[test]
fn a_node_extension_names_a_js_or_mjs_script() {
    for name in ["by-code.js", "by-code.mjs"] {
        let files = with_rule(
            &implementation("node", &format!("file = \"{name}\"\n")),
            vec![script(name, "console.log(1)\n")],
        );

        let rule = loaded(&files)
            .resolve("acme/by-code")
            .cloned()
            .expect("rule");

        assert_eq!(
            rule.implementations[0].source.as_deref(),
            Some("console.log(1)\n"),
            "{name}"
        );
    }
}

#[test]
fn the_script_has_the_extension_of_its_runtime() {
    for (runtime, name) in [
        ("python", "by-code.js"),
        ("python", "by-code.sh"),
        ("node", "by-code.py"),
        ("node", "by-code.ts"),
    ] {
        let files = with_rule(
            &implementation(runtime, &format!("file = \"{name}\"\n")),
            vec![script(name, "x\n")],
        );

        let message = rejected(&files);

        assert!(message.contains(name), "{runtime} {name}: {message}");
        assert!(message.contains(runtime), "{runtime} {name}: {message}");
    }
}

#[test]
fn a_script_is_a_file_of_the_folder_of_its_rule() {
    for name in [
        "sub/by-code.py",
        "../by-code.py",
        "/by-code.py",
        ".py",
        "sub\\\\by-code.py",
    ] {
        let files = with_rule(
            &implementation("python", &format!("file = \"{name}\"\n")),
            vec![],
        );

        let message = rejected(&files);

        assert!(
            message.contains("a file of the folder of the rule"),
            "{name}: {message}"
        );
    }
}

#[test]
fn a_python_or_node_extension_needs_its_file_and_the_file_must_be_there() {
    for runtime in ["python", "node"] {
        let named = if runtime == "python" {
            "file = \"by-code.py\"\n"
        } else {
            "file = \"by-code.mjs\"\n"
        };
        let missing = rejected(&with_rule(&implementation(runtime, named), vec![]));
        assert!(missing.contains("is missing"), "{runtime}: {missing}");

        let unnamed = rejected(&with_rule(&implementation(runtime, ""), vec![]));
        assert!(
            unnamed.contains("must name its script with `file`"),
            "{runtime}: {unnamed}"
        );
    }
}

#[test]
fn a_runtime_on_another_kind_is_reported_once_and_with_no_word_about_scripts() {
    let ast_grep = "[[implementations]]\nkind = \"ast-grep\"\nruntime = \"python\"\n\
                    languages = [\"python\"]\nfile = \"python.yml\"\n";

    let message = rejected(&with_rule(
        ast_grep,
        vec![script("python.yml", "id: acme/by-code\n")],
    ));

    assert!(
        message.contains("runtime only applies to extensions"),
        "{message}"
    );
    assert!(!message.contains("script"), "{message}");
}

#[test]
fn a_command_extension_names_its_command_and_no_file() {
    let files = with_rule(
        &implementation("command", "command = [\"sh\", \"check.sh\", \"--all\"]\n"),
        vec![],
    );

    let rule = loaded(&files)
        .resolve("acme/by-code")
        .cloned()
        .expect("rule");

    assert_eq!(
        rule.implementations[0].command.as_deref(),
        Some(&["sh".to_owned(), "check.sh".to_owned(), "--all".to_owned()][..])
    );
    assert_eq!(rule.implementations[0].source, None);
}

#[test]
fn a_command_extension_needs_a_command_with_a_program() {
    for fields in ["", "command = []\n", "command = [\"\"]\n"] {
        let message = rejected(&with_rule(&implementation("command", fields), vec![]));

        assert!(message.contains("needs a `command`"), "{fields}: {message}");
    }
}

#[test]
fn a_command_extension_has_no_file() {
    let files = with_rule(
        &implementation("command", "command = [\"sh\"]\nfile = \"x.py\"\n"),
        vec![script("x.py", "x\n")],
    );

    let message = rejected(&files);

    assert!(
        message.contains("takes a `command` and no `file`"),
        "{message}"
    );
}

#[test]
fn only_a_command_extension_has_a_command() {
    for runtime in ["python", "node"] {
        let files = with_rule(
            &implementation(runtime, "file = \"by-code.py\"\ncommand = [\"sh\"]\n"),
            vec![script("by-code.py", "x\n")],
        );

        let message = rejected(&files);

        assert!(
            message.contains("`command` applies to the runtime `command` only"),
            "{runtime}: {message}"
        );
    }
    let ast_grep = "[[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\n\
                    file = \"python.yml\"\ncommand = [\"sh\"]\n";
    let message = rejected(&with_rule(
        ast_grep,
        vec![script("python.yml", "id: acme/by-code\n")],
    ));
    assert!(
        message.contains("`command` applies to the runtime `command` only"),
        "{message}"
    );
}

#[test]
fn the_files_next_to_a_script_are_not_the_catalog_s_business() {
    let files = with_rule(
        &implementation("python", "file = \"by-code.py\"\n"),
        vec![
            script("by-code.py", "import helpers\n"),
            script("helpers.py", "X = 1\n"),
            script("lib.mjs", "export {}\n"),
        ],
    );

    let rule = loaded(&files)
        .resolve("acme/by-code")
        .cloned()
        .expect("rule");

    assert_eq!(
        rule.implementations[0].source.as_deref(),
        Some("import helpers\n")
    );
}

#[test]
fn the_built_in_catalog_takes_no_extension_whatever_its_runtime() {
    for (runtime, fields, script_name) in [
        ("python", "file = \"by-code.py\"\n", "by-code.py"),
        ("node", "file = \"by-code.mjs\"\n", "by-code.mjs"),
        ("command", "command = [\"sh\"]\n", "unused.py"),
    ] {
        let files = [
            vec![family("structure", "STRUC")],
            common::rule_with(
                "structure",
                "by-code",
                "BTSTRUC001",
                "",
                &implementation(runtime, fields),
            ),
            vec![file(&format!("structure/by-code/{script_name}"), "x\n")],
        ]
        .concat();

        let message = Catalog::from_files(&files)
            .expect_err("an extension is not for the built-in catalog")
            .to_string();

        assert!(
            message.contains("`by-code` is an extension"),
            "{runtime}: {message}"
        );
    }
}
