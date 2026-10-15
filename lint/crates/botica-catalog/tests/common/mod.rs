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
