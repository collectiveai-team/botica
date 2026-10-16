//! A catalog that the configuration declares, `[catalogs] <name> = { path = "…" }`, and the extra
//! folders of catalogs of `rule-dirs`, load as the catalogs of `.botica/rules` do.

mod common;

use std::path::{Path, PathBuf};

use common::{command, project_in, stderr, stdout, write};

const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// The files of the catalog `name` with the code `code`, whose rule `no-eval` flags `eval(…)`,
/// written into `catalog` (the folder of the catalog itself).
fn lay_out_catalog(catalog: &Path, name: &str, code: &str) {
    write(catalog, "catalog.toml", &format!("code = \"{code}\"\n"));
    write(
        catalog,
        "structure/family.toml",
        "code = \"STRUC\"\ndescription = \"d\"\n",
    );
    let rule = "structure/no-eval";
    write(
        catalog,
        &format!("{rule}/rule.toml"),
        &format!(
            "id = \"{name}/no-eval\"\ncode = \"{code}STRUC001\"\nsummary = \"Do not call eval\"\n\
             status = \"stable\"\nscope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\n\
             default-enabled = true\ndefault-severity = \"error\"\n\n[[implementations]]\n\
             kind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n"
        ),
    );
    write(catalog, &format!("{rule}/README.md"), README);
    write(
        catalog,
        &format!("{rule}/python.yml"),
        &format!(
            "id: {name}/no-eval\nlanguage: python\nseverity: error\n\
             message: \"{code}STRUC001 ({name}/no-eval): do not call eval\"\n\
             rule:\n  pattern: eval($$$ARGS)\n"
        ),
    );
}

fn project(name: &str, config: &str) -> PathBuf {
    let dir = project_in("declared-catalogs", name, Some(config));
    let dir = std::fs::canonicalize(dir).expect("the project exists");
    write(&dir, "app.py", "eval(\"1\")\n");
    dir
}

/// The ids of the rules that reported.
fn reported(dir: &Path) -> Vec<String> {
    stdout(&command(dir, &["lint", "."]))
        .lines()
        .filter_map(|line| line.split_once('[')?.1.split_once(']'))
        .map(|(rule, _)| rule.to_owned())
        .collect()
}

#[test]
fn a_catalog_declared_by_path_runs_its_rules_under_its_name() {
    let dir = project(
        "by-path",
        "[catalogs]\nshared = { path = \"vendor/shared\" }\n",
    );
    lay_out_catalog(&dir.join("vendor/shared"), "shared", "SHARED");

    assert_eq!(reported(&dir), ["shared/no-eval"]);
}

#[test]
fn the_fixtures_of_a_catalog_inside_the_project_are_not_linted_wherever_the_catalog_is() {
    let dir = project(
        "fixtures",
        "rule-dirs = [\".botica/rules\", \"team-rules\"]\n[catalogs]\nshared = { path = \"vendor/shared\" }\n",
    );
    for (catalog, name, code) in [
        ("vendor/shared", "shared", "SHARED"),
        ("team-rules/team", "team", "TEAM"),
    ] {
        lay_out_catalog(&dir.join(catalog), name, code);
        write(
            &dir,
            &format!("{catalog}/structure/no-eval/fixtures/invalid/bad.py"),
            "eval(\"1\")\n",
        );
    }

    assert_eq!(reported(&dir), ["shared/no-eval", "team/no-eval"]);
}

#[test]
fn a_declared_path_may_be_outside_the_project() {
    let dir = project(
        "outside",
        "[catalogs]\nshared = { path = \"../outside-shared\" }\n",
    );
    lay_out_catalog(
        &dir.parent().expect("a parent").join("outside-shared"),
        "shared",
        "SHARED",
    );

    assert_eq!(reported(&dir), ["shared/no-eval"]);
}

#[test]
fn a_declared_catalog_whose_folder_is_missing_is_an_error_that_names_the_path() {
    let dir = project(
        "missing",
        "[catalogs]\nshared = { path = \"vendor/nope\" }\n",
    );

    let output = command(&dir, &["lint", "."]);

    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("`vendor/nope`, which is not a folder"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_catalog_in_a_folder_of_rule_dirs_and_a_declared_one_do_not_share_a_name() {
    let dir = project("clash", "[catalogs]\nacme = { path = \"vendor/acme\" }\n");
    lay_out_catalog(&dir.join("vendor/acme"), "acme", "ACME");
    lay_out_catalog(&dir.join(".botica/rules/acme"), "acme", "ACME");

    let output = command(&dir, &["lint", "."]);

    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("`acme`") && stderr(&output).contains("more than once"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn rule_dirs_names_every_folder_that_holds_catalogs() {
    let dir = project(
        "rule-dirs",
        "rule-dirs = [\".botica/rules\", \"team-rules\"]\n",
    );
    lay_out_catalog(&dir.join(".botica/rules/acme"), "acme", "ACME");
    lay_out_catalog(&dir.join("team-rules/team"), "team", "TEAM");

    assert_eq!(reported(&dir), ["acme/no-eval", "team/no-eval"]);
}

#[test]
fn a_folder_that_rule_dirs_does_not_name_holds_no_catalog() {
    let dir = project("rule-dirs-only", "rule-dirs = [\"team-rules\"]\n");
    lay_out_catalog(&dir.join(".botica/rules/acme"), "acme", "ACME");

    assert_eq!(reported(&dir), Vec::<String>::new());
}

#[test]
fn a_declared_catalog_cannot_take_the_name_of_a_built_in_family() {
    let dir = project(
        "reserved",
        "[catalogs]\nstructure = { path = \"vendor/structure\" }\n",
    );
    lay_out_catalog(&dir.join("vendor/structure"), "structure", "SHARED");

    let output = command(&dir, &["lint", "."]);

    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("family of the built-in catalog"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn the_script_of_an_extension_rule_is_read_from_the_folder_of_its_declared_catalog() {
    if !common::node::installed() {
        return;
    }
    let dir = project(
        "script",
        "[catalogs]\nshared = { path = \"vendor/shared\" }\n",
    );
    let catalog = dir.join("vendor/shared");
    lay_out_catalog(&catalog, "shared", "SHARED");
    let rule = "structure/no-eval";
    write(
        &catalog,
        &format!("{rule}/rule.toml"),
        "id = \"shared/no-eval\"\ncode = \"SHAREDSTRUC001\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         scope = \"file\"\nneeds = \"syntax\"\nrun-on = [\"cli\"]\ndefault-enabled = true\n\
         default-severity = \"error\"\n\n[[implementations]]\nkind = \"extension\"\n\
         runtime = \"node\"\nlanguages = [\"python\"]\nfile = \"check.mjs\"\n",
    );
    std::fs::remove_file(catalog.join(rule).join("python.yml")).expect("remove the YAML");
    write(
        &catalog,
        &format!("{rule}/check.mjs"),
        r#"import { createInterface } from "node:readline";
const reply = (id, result) => console.log(JSON.stringify({ jsonrpc: "2.0", id, result }));
for await (const line of createInterface({ input: process.stdin })) {
  const request = JSON.parse(line);
  if (request.method === "initialize") reply(request.id, { protocolVersion: 1, rules: [] });
  else if (request.method === "lint/files") {
    const diagnostics = request.params.files.map((file) => ({
      path: file.path, rule: "shared/no-eval", severity: "error", start: 0, end: 1, message: "seen",
    }));
    reply(request.id, { diagnostics });
  } else if (request.method === "shutdown") { reply(request.id, null); break; }
}
process.stdin.destroy();
"#,
    );

    assert_eq!(reported(&dir), ["shared/no-eval"]);
}
