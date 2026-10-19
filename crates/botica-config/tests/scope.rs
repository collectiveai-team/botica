use botica_catalog::{Catalog, Scope, SourceFile};
use botica_config::{Config, Reason, Resolution};

fn rule_toml(slug: &str, code: &str, scope: &str) -> String {
    format!(
        "id = \"{slug}\"\ncode = \"{code}\"\nsummary = \"s\"\nstatus = \"stable\"\n\
         scope = \"{scope}\"\nneeds = \"path\"\nrun-on = [\"cli\"]\ndefault-enabled = true\n\
         default-severity = \"error\"\n[[implementations]]\nkind = \"native\"\nlanguages = [\"*\"]\n"
    )
}

fn catalog() -> Catalog {
    let mut files = vec![SourceFile {
        path: "structure/family.toml".into(),
        text: "code = \"STRUC\"\ndescription = \"d\"\n".into(),
    }];
    for (slug, code, scope) in [
        ("per-file", "BTSTRUC001", "file"),
        ("whole-project", "BTSTRUC002", "project"),
    ] {
        files.push(SourceFile {
            path: format!("structure/{slug}/rule.toml"),
            text: rule_toml(slug, code, scope),
        });
        files.push(SourceFile {
            path: format!("structure/{slug}/README.md"),
            text: "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n".into(),
        });
    }
    files.push(SourceFile {
        path: "structure/by-a-tool/rule.toml".into(),
        text: "id = \"by-a-tool\"\ncode = \"BTSTRUC003\"\nsummary = \"s\"\nstatus = \"stable\"\n\
               default-enabled = true\ndefault-severity = \"error\"\n[[implementations]]\n\
               kind = \"external\"\nlanguages = [\"python\"]\ntool = \"complexipy\"\n\
               docs = \"https://rohaquinlop.github.io/complexipy/\"\n"
            .into(),
    });
    files.push(SourceFile {
        path: "structure/by-a-tool/README.md".into(),
        text: "## Directive\n\nd\n\n## Why\n\nw\n\n## Tool\n\nt\n\n## Configuration\n\nc\n\n\
               ## Do\n\ndo\n\n## Don\'t\n\ndont\n\n## Suppression\n\ns\n"
            .into(),
    });
    Catalog::from_files(&files).unwrap_or_else(|error| panic!("catalog:\n{error}"))
}

fn active(scope: Option<Scope>) -> (Vec<String>, Resolution) {
    let catalog = catalog();
    let resolution = Resolution::new(&Config::default(), &catalog)
        .expect("valid config")
        .only_scope(scope);
    let mut slugs: Vec<String> = resolution
        .entries()
        .iter()
        .filter(|entry| entry.active)
        .map(|entry| entry.rule.id.clone())
        .collect();
    slugs.sort();
    (slugs, resolution)
}

#[test]
fn without_a_scope_every_rule_runs() {
    assert_eq!(active(None).0, ["by-a-tool", "per-file", "whole-project"]);
}

#[test]
fn a_scope_keeps_only_the_rules_of_that_scope_and_those_that_lint_does_not_run() {
    assert_eq!(active(Some(Scope::File)).0, ["by-a-tool", "per-file"]);
    assert_eq!(
        active(Some(Scope::Project)).0,
        ["by-a-tool", "whole-project"]
    );
}

#[test]
fn a_rule_left_out_by_the_scope_says_so() {
    let (_, resolution) = active(Some(Scope::Project));
    let entry = resolution.entry("per-file").expect("rule");
    assert_eq!(entry.reason, Reason::OutOfScope("project".to_owned()));
    assert_eq!(entry.reason.to_string(), "outside `--scope project`");
}

#[test]
fn a_rule_that_was_already_inactive_keeps_its_reason() {
    let catalog = catalog();
    let config: Config = toml::from_str("ignore = [\"per-file\"]\n").expect("config");
    let resolution = Resolution::new(&config, &catalog)
        .expect("valid config")
        .only_scope(Some(Scope::Project));
    assert_eq!(
        resolution.entry("per-file").expect("rule").reason,
        Reason::IgnoredBy("per-file".to_owned())
    );
}
