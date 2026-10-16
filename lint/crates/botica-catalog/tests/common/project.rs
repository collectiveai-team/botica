//! What the tests of the catalogs of a project share: a built-in catalog and the catalog `acme`.

use botica_catalog::{Catalog, SourceFile};

use super::{family, file, rule, rule_toml, README};

pub const AST_GREP_PYTHON: &str =
    "[[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n";

pub fn built_in() -> Catalog {
    Catalog::from_files(
        &[
            vec![family("structure", "STRUC")],
            rule("structure", "no-utils", "BTSTRUC001"),
        ]
        .concat(),
    )
    .expect("the built-in catalog loads")
}

/// `<name>/catalog.toml`, the file that makes a folder a catalog.
pub fn catalog_toml(name: &str, code: &str) -> SourceFile {
    file(
        &format!("{name}/catalog.toml"),
        &format!("code = \"{code}\"\n"),
    )
}

/// A rule of a family of a project catalog: its id is `<catalog>/<slug>`.
pub fn project_rule(
    name: &str,
    family_dir: &str,
    slug: &str,
    code: &str,
    extra: &str,
    implementation: &str,
) -> Vec<SourceFile> {
    vec![
        file(
            &format!("{name}/{family_dir}/{slug}/rule.toml"),
            &rule_toml(&format!("{name}/{slug}"), code, extra, implementation),
        ),
        file(&format!("{name}/{family_dir}/{slug}/README.md"), README),
    ]
}

pub fn native() -> &'static str {
    super::NATIVE_ALL
}

pub fn acme() -> Vec<SourceFile> {
    [
        vec![
            catalog_toml("acme", "ACME"),
            family("acme/structure", "STRUC"),
        ],
        project_rule(
            "acme",
            "structure",
            "no-blobs",
            "ACMESTRUC001",
            "",
            AST_GREP_PYTHON,
        ),
        vec![file(
            "acme/structure/no-blobs/python.yml",
            "id: acme/no-blobs\n",
        )],
    ]
    .concat()
}

pub fn rejected(files: &[SourceFile]) -> String {
    built_in()
        .with_projects(files)
        .expect_err("the project catalog should be rejected")
        .to_string()
}
