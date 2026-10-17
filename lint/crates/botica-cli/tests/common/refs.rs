//! What the tests of `botica refs` share: a project with the topic of a file, and the commands.

use std::path::{Path, PathBuf};
use std::process::Output;

use super::{command, project_in, stderr, stdout, write};

/// The configuration that lets a topic of the project replace the built-in `structured-logging`.
pub const REPLACE_LOGGING: &str = "[references]\nreplace = [\"structured-logging\"]\n";

pub const SOURCE: &str = "import os\n\ndef get_logger():\n    pass\n\nTAIL = 1\n";

/// A reference to a page of the web, to append to `topic_toml`. The host never resolves: a command
/// that fetched it would fail.
pub const PAGE: &str = "\n[[references]]\nurl = \"https://example.invalid/docs#setup\"\n\
                    note = \"the documentation of the library\"\n";

pub fn topic_toml(extra: &str) -> String {
    format!(
        "title = \"Demo topic\"\n\n\
         [[references]]\npath = \"src/app.py\"\nlines = \"3-4\"\nanchor = \"def get_logger\"\n\
         note = \"the setup\"\n{extra}"
    )
}

pub fn add_topic(dir: &Path, id: &str, toml: &str) {
    write(dir, &format!("references/{id}/topic.toml"), toml);
    write(
        dir,
        &format!("references/{id}/README.md"),
        "Read `get_logger`.\n",
    );
}

/// A project with the file `src/app.py` and the topic `id`, which points at it. The id of a built-in
/// topic is replaced, as the configuration of the project says.
pub fn project(name: &str, id: &str) -> PathBuf {
    let config = (id == "structured-logging").then_some(REPLACE_LOGGING);
    let dir = project_in("refs", name, config);
    write(&dir, "src/app.py", SOURCE);
    add_topic(&dir, id, &topic_toml(""));
    dir
}

pub fn standard(dir: &Path, number: &str, references: &str) {
    write(
        dir,
        &format!("standards/CES/{number}-a-standard.md"),
        &format!(
            "---\nid: CES-{number}\ntitle: A standard\nreferences: {references}\n---\n\nText.\n"
        ),
    );
}

pub fn refs(dir: &Path, args: &[&str]) -> Output {
    let mut all = vec!["refs"];
    all.extend_from_slice(args);
    command(dir, &all)
}

pub fn ok(output: &Output) -> String {
    assert!(output.status.success(), "{}", stderr(output));
    stdout(output)
}

pub fn failed(output: &Output, code: i32) -> String {
    assert_eq!(output.status.code(), Some(code), "{}", stdout(output));
    format!("{}{}", stdout(output), stderr(output))
}

pub const RULE_README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// A catalog of the project, `acme`, whose rule `acme/no-eval` cites `references`.
pub fn acme_rule(dir: &Path, references: &str) {
    let root = ".botica/rules/acme";
    write(dir, &format!("{root}/catalog.toml"), "code = \"ACME\"\n");
    write(
        dir,
        &format!("{root}/structure/family.toml"),
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = format!("{root}/structure/no-eval");
    write(
        dir,
        &format!("{rule}/rule.toml"),
        &format!(
            "id = \"acme/no-eval\"\ncode = \"ACMESTRUC001\"\nsummary = \"Do not call eval\"\n\
             status = \"stable\"\nreferences = {references}\nscope = \"file\"\nneeds = \"syntax\"\n\
             run-on = [\"cli\"]\ndefault-enabled = true\ndefault-severity = \"error\"\n\n\
             [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n"
        ),
    );
    write(dir, &format!("{rule}/README.md"), RULE_README);
    write(
        dir,
        &format!("{rule}/python.yml"),
        "id: acme/no-eval\nlanguage: python\nseverity: error\nmessage: \"ACMESTRUC001 (acme/no-eval): no\"\nrule:\n  pattern: eval($$$ARGS)\n",
    );
    write(dir, &format!("{rule}/fixtures/invalid/bad.py"), "eval(x)\n");
    write(dir, &format!("{rule}/fixtures/valid/ok.py"), "x = 1\n");
    write(
        dir,
        &format!("{rule}/fixtures/expected.json"),
        r#"{"diagnostics":[{"path":"bad.py","severity":"error","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":7,"line":1,"column":8}},"message":"ACMESTRUC001 (acme/no-eval): no"}]}"#,
    );
}
