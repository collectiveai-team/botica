//! `botica server`: an editor opens a buffer that is not on disk and gets the findings of the
//! rules of the project.

mod common;

use common::lsp::{initialize, open, Session};
use common::{project_in, write};
use serde_json::{json, Value};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

fn lay_out_acme(dir: &std::path::Path) {
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
        "id = \"acme/no-eval\"\ncode = \"ACMESTRUC001\"\nsummary = \"Do not call eval\"\n\
         status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\n\
         default-enabled = true\ndefault-severity = \"error\"\n\n[[implementations]]\n\
         kind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n",
    );
    write(dir, &format!("{rule}/README.md"), README);
    write(
        dir,
        &format!("{rule}/python.yml"),
        "id: acme/no-eval\nlanguage: python\nseverity: error\n\
         message: \"ACMESTRUC001 (acme/no-eval): do not call eval\"\n\
         rule:\n  pattern: eval($$$ARGS)\n",
    );
}

#[test]
fn a_buffer_that_is_not_on_disk_gets_the_findings_of_the_rules_of_the_project() {
    let dir = project_in("server", "buffer", Some(""));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    lay_out_acme(&dir);
    let uri = format!("file://{}/scratch.py", dir.display());

    let mut editor = Session::start(&dir, &["server"]);
    editor.send(&initialize(&Value::Null));
    let initialized = editor.read();
    editor.send(&open(&uri, "x = 1\neval(\"1\")\n"));
    let published = editor.read();
    let (rest, status) = editor.finish();

    assert_eq!(initialized["result"]["serverInfo"]["name"], "botica");
    assert_eq!(published["method"], "textDocument/publishDiagnostics");
    assert_eq!(published["params"]["uri"], uri);
    let found = &published["params"]["diagnostics"][0];
    assert_eq!(found["code"], "acme/no-eval");
    assert_eq!(found["severity"], 1);
    assert_eq!(found["range"]["start"], json!({"line": 1, "character": 0}));
    assert_eq!(
        published["params"]["diagnostics"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(rest[0]["result"], Value::Null);
    assert!(status.success());
}
