#![allow(dead_code)]

use botica_catalog::{Catalog, SourceFile};

pub fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

pub fn family(dir: &str, code: &str) -> SourceFile {
    file(
        &format!("{dir}/family.toml"),
        &format!("code = \"{code}\"\ndescription = \"A family used by the tests\"\n"),
    )
}

pub fn rule_toml(slug: &str, code: &str, extra: &str, implementation: &str) -> String {
    format!(
        r#"id = "{slug}"
code = "{code}"
summary = "A rule used by the tests"
status = "stable"
scope = "file"
needs = "bytes"
run-on = ["cli"]
default-enabled = true
default-severity = "error"
{extra}
{implementation}"#
    )
}

/// A README with the required sections of a rule of a family.
pub const README: &str =
    "## Directive\n\nd\n\n## Why\n\nw\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n## Suppression\n\ns\n";

/// A README with the sections of a rule of botica itself.
pub const OWN_README: &str = "## Directive\n\nd\n\n## Why\n\nw\n\n## Suppression\n\ns\n";

pub const NATIVE_ALL: &str = "[[implementations]]\nkind = \"native\"\nlanguages = [\"*\"]\n";

pub fn rule(dir: &str, slug: &str, code: &str) -> Vec<SourceFile> {
    rule_with(dir, slug, code, "", NATIVE_ALL)
}

pub fn rule_with(
    dir: &str,
    slug: &str,
    code: &str,
    extra: &str,
    implementation: &str,
) -> Vec<SourceFile> {
    vec![
        file(
            &format!("{dir}/{slug}/rule.toml"),
            &rule_toml(slug, code, extra, implementation),
        ),
        file(
            &format!("{dir}/{slug}/README.md"),
            if dir == "botica" { OWN_README } else { README },
        ),
    ]
}

pub fn catalog(parts: Vec<Vec<SourceFile>>) -> Result<Catalog, botica_catalog::CatalogError> {
    Catalog::from_files(&parts.concat())
}

pub fn issues(parts: Vec<Vec<SourceFile>>) -> String {
    catalog(parts)
        .expect_err("the catalog should be rejected")
        .to_string()
}

/// A README with the sections of a judgment rule.
pub const JUDGMENT_README: &str = "## Directive\n\nd\n\n## Why\n\nw\n\n## Flag\n\nf\n\n\
## Do not flag\n\nn\n\n## Confirm\n\nc\n\n## Do\n\ndo\n\n## Don't\n\ndont\n\n\
## Suppression\n\ns\n";

/// The root of the `rule.toml` of `test-through-interface`, a judgment rule of `testing`.
pub const JUDGMENT_HEAD: &str = "id = \"test-through-interface\"\ncode = \"BTTEST001\"\n\
summary = \"Tests assert behaviour through the public seam\"\nstatus = \"preview\"\n\
default-enabled = false\ndefault-severity = \"warning\"\n";

/// The fields of the root of a judgment implementation, before the table of its question.
pub const JUDGMENT_FIELDS: &str =
    "[[implementations]]\nkind = \"judgment\"\nlanguages = [\"python\"]\nwhen = \"edit\"\n\
why-not-deterministic = \"Whether a seam is public depends on the design of the module.\"\n";

/// A boolean question, the table after the fields of a judgment implementation.
pub const BOOLEAN_QUESTION: &str = "\n[implementations.question]\ntype = \"boolean\"\n\
instructions = \"Does the test assert through the public seam?\"\n";

pub fn judgment(head_extra: &str, implementation: &str) -> Vec<SourceFile> {
    vec![
        family("testing", "TEST"),
        file(
            "testing/test-through-interface/rule.toml",
            &format!("{JUDGMENT_HEAD}{head_extra}\n{implementation}"),
        ),
        file("testing/test-through-interface/README.md", JUDGMENT_README),
    ]
}

pub fn judgment_implementation() -> String {
    format!("{JUDGMENT_FIELDS}{BOOLEAN_QUESTION}")
}

pub mod project;
