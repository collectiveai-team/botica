//! The conformance runner checks an extension rule of a catalog of a project with its fixtures, as
//! any other rule.

mod common;

use std::path::{Path, PathBuf};

use botica_conformance::Runner;
use common::write;

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

const EXPECTED: &str = r#"{"diagnostics":[{"path":"bad.py","severity":"error","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":1,"line":1,"column":2}},"message":"seen"}]}"#;

/// An extension that answers with a finding of `acme/by-sh` for each file whose name `flags`.
fn script(flags: &str) -> String {
    format!(
        r#"while IFS= read -r line; do
  case "$line" in
    *'"initialize"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":1,"result":{{"rules":[]}}}}' ;;
    *'"lint/files"'*)
      items=""
      for path in $(printf '%s' "$line" | grep -o '"path":"[^"]*"' | sed 's/"path":"//; s/"$//'); do
        case "$path" in
          {flags})
            one='{{"path":"'"$path"'","rule":"acme/by-sh","severity":"error","start":0,"end":1,"message":"seen"}}'
            items="${{items:+$items,}}$one" ;;
        esac
      done
      printf '%s\n' '{{"jsonrpc":"2.0","id":2,"result":{{"diagnostics":['"$items"']}}}}' ;;
    *'"shutdown"'*) printf '%s\n' '{{"jsonrpc":"2.0","id":3,"result":null}}'; exit 0 ;;
  esac
done
"#
    )
}

/// `<scratch>/acme`, a catalog with one extension rule, `acme/by-sh`, that flags the files given.
fn catalog(name: &str, flags: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conformance-project")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    let acme = dir.join("acme");
    write(&acme, "catalog.toml", "code = \"ACME\"\n");
    write(
        &acme,
        "structure/family.toml",
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = "structure/by-sh";
    write(
        &acme,
        &format!("{rule}/rule.toml"),
        "id = \"acme/by-sh\"\ncode = \"ACMESTRUC001\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         scope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\ndefault-enabled = false\n\
         default-severity = \"error\"\n\n[[implementations]]\nkind = \"extension\"\n\
         runtime = \"command\"\nlanguages = [\"python\"]\n\
         command = [\"sh\", \".botica/rules/acme/structure/by-sh/ext.sh\"]\n",
    );
    write(&acme, &format!("{rule}/README.md"), README);
    write(&acme, &format!("{rule}/ext.sh"), &script(flags));
    write(&acme, &format!("{rule}/fixtures/valid/ok.py"), "x = 1\n");
    write(&acme, &format!("{rule}/fixtures/invalid/bad.py"), "x = 2\n");
    write(&acme, &format!("{rule}/fixtures/expected.json"), EXPECTED);
    acme
}

fn runner(name: &str) -> Runner {
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("conformance-project")
        .join(format!("{name}-work"));
    Runner::new(env!("CARGO_BIN_EXE_botica"), scratch)
}

#[test]
fn an_extension_rule_of_a_project_catalog_conforms_to_its_fixtures() {
    let acme = catalog("passes", "bad.py");

    let outcomes = runner("passes")
        .check_project_catalog(&acme)
        .expect("the catalog is checked");

    let names: Vec<&str> = outcomes.iter().map(|outcome| outcome.rule()).collect();
    assert_eq!(names, ["acme/by-sh"]);
    let reasons: Vec<String> = outcomes[0]
        .failures()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(outcomes[0].passed(), "{}", reasons.join("\n"));
}

#[test]
fn a_finding_on_a_valid_fixture_or_a_missing_one_on_an_invalid_fixture_fails_the_rule() {
    for (name, flags) in [
        ("flags-valid", "bad.py|ok.py"),
        ("flags-nothing", "other.py"),
    ] {
        let acme = catalog(name, flags);

        let outcomes = runner(name)
            .check_project_catalog(&acme)
            .expect("the catalog is checked");

        assert!(!outcomes[0].passed(), "{name}");
    }
}
