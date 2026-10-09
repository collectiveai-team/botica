use botica_catalog::{Catalog, SourceFile, Status};

fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.to_owned(),
        text: text.to_owned(),
    }
}

fn family(dir: &str, code: &str) -> SourceFile {
    file(
        &format!("{dir}/family.toml"),
        &format!("code = \"{code}\"\ndescription = \"A family used by the tests\"\n"),
    )
}

fn rule_toml(slug: &str, code: &str, extra: &str, implementation: &str) -> String {
    format!(
        r#"id = "{slug}"
code = "{code}"
summary = "A rule used by the tests"
status = "stable"
since = "0.1.0"
scope = "file"
needs = "bytes"
run-on = ["cli"]
default-enabled = true
default-severity = "error"
{extra}
{implementation}"#
    )
}

const NATIVE_ALL: &str = "[[implementations]]\nkind = \"native\"\nlanguages = [\"*\"]\n";

fn rule(dir: &str, slug: &str, code: &str) -> Vec<SourceFile> {
    rule_with(dir, slug, code, "", NATIVE_ALL)
}

fn rule_with(
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
        file(&format!("{dir}/{slug}/README.md"), "# readme\n"),
    ]
}

fn catalog(parts: Vec<Vec<SourceFile>>) -> Result<Catalog, botica_catalog::CatalogError> {
    Catalog::from_files(&parts.concat())
}

fn issues(parts: Vec<Vec<SourceFile>>) -> String {
    catalog(parts)
        .expect_err("the catalog should be rejected")
        .to_string()
}

#[test]
fn loads_families_and_rules_and_resolves_slug_code_and_alias() {
    let loaded = catalog(vec![
        vec![family("structure", "STRUC")],
        rule_with(
            "structure",
            "file-size-guard",
            "BTSTRUC001",
            "aliases = [\"size-guard\"]",
            NATIVE_ALL,
        ),
        rule("botica", "io-error", "BT001"),
    ])
    .expect("a valid catalog loads");

    assert_eq!(loaded.rules().len(), 2);
    let by_slug = loaded.resolve("file-size-guard").expect("slug resolves");
    assert_eq!(by_slug.code, "BTSTRUC001");
    assert_eq!(by_slug.family.as_deref(), Some("structure"));
    assert_eq!(
        loaded.resolve("BTSTRUC001").map(|r| r.id.as_str()),
        Some("file-size-guard")
    );
    assert_eq!(
        loaded.resolve("size-guard").map(|r| r.id.as_str()),
        Some("file-size-guard")
    );
    assert_eq!(loaded.resolve("BT001").and_then(|r| r.family.clone()), None);
    assert!(loaded.resolve("nope").is_none());
}

#[test]
fn rejects_a_code_without_the_prefix_of_its_family() {
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule("structure", "file-size-guard", "BTARCH001"),
    ]);
    assert!(message.contains("BTSTRUC"), "{message}");
}

#[test]
fn rejects_a_code_that_does_not_end_in_three_digits() {
    for bad in ["BTSTRUC01", "BTSTRUC0001", "BTSTRUC", "STRUC001"] {
        let message = issues(vec![
            vec![family("structure", "STRUC")],
            rule("structure", "file-size-guard", bad),
        ]);
        assert!(message.contains("code"), "{bad}: {message}");
    }
}

#[test]
fn rejects_a_family_with_a_digit_or_lowercase_in_its_code() {
    for bad in ["K8S", "struc", "ST-RUC", ""] {
        let message = issues(vec![vec![family("structure", bad)]]);
        assert!(message.contains("family code"), "{bad}: {message}");
    }
}

#[test]
fn rejects_duplicate_slugs_across_families() {
    let message = issues(vec![
        vec![family("structure", "STRUC"), family("testing", "TEST")],
        rule("structure", "same-name", "BTSTRUC001"),
        rule("testing", "same-name", "BTTEST001"),
    ]);
    assert!(
        message.contains("same-name") && message.contains("more than one"),
        "{message}"
    );
}

#[test]
fn rejects_duplicate_rule_codes() {
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule("structure", "one", "BTSTRUC001"),
        rule("structure", "two", "BTSTRUC001"),
    ]);
    assert!(
        message.contains("BTSTRUC001") && message.contains("already used"),
        "{message}"
    );
}

#[test]
fn rejects_duplicate_family_codes() {
    let message = issues(vec![vec![
        family("structure", "STRUC"),
        family("architecture", "STRUC"),
    ]]);
    assert!(
        message.contains("STRUC") && message.contains("already used"),
        "{message}"
    );
}

#[test]
fn a_family_code_may_be_a_prefix_of_another() {
    catalog(vec![
        vec![family("process", "P"), family("prefect", "PREF")],
        rule("process", "one", "BTP001"),
        rule("prefect", "two", "BTPREF001"),
    ])
    .expect("P and PREF are different families: the code is split into letters and digits");
}

#[test]
fn rejects_a_rule_folder_without_family_toml() {
    let message = issues(vec![rule("structure", "file-size-guard", "BTSTRUC001")]);
    assert!(message.contains("family.toml"), "{message}");
}

#[test]
fn the_linters_own_rules_take_no_family_and_use_bt_plus_three_digits() {
    let message = issues(vec![rule("botica", "io-error", "BTIO001")]);
    assert!(message.contains("BT"), "{message}");

    let message = issues(vec![
        vec![family("botica", "BOT")],
        rule("botica", "io-error", "BT001"),
    ]);
    assert!(
        message.contains("botica") && message.contains("family.toml"),
        "{message}"
    );
}

#[test]
fn rejects_an_id_that_differs_from_its_folder() {
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        vec![
            file(
                "structure/folder-name/rule.toml",
                &rule_toml("other-name", "BTSTRUC001", "", NATIVE_ALL),
            ),
            file("structure/folder-name/README.md", "# readme\n"),
        ],
    ]);
    assert!(
        message.contains("folder-name") && message.contains("other-name"),
        "{message}"
    );
}

#[test]
fn rejects_an_alias_that_collides_with_a_slug_a_code_or_another_alias() {
    for (clash, other_aliases) in [
        ("other", ""),
        ("BTSTRUC002", ""),
        ("shared", "aliases = [\"shared\"]"),
    ] {
        let message = issues(vec![
            vec![family("structure", "STRUC")],
            rule_with(
                "structure",
                "one",
                "BTSTRUC001",
                &format!("aliases = [\"{clash}\"]"),
                NATIVE_ALL,
            ),
            rule_with(
                "structure",
                "other",
                "BTSTRUC002",
                other_aliases,
                NATIVE_ALL,
            ),
        ]);
        assert!(
            message.contains(&format!("alias `{clash}`")),
            "{clash}: {message}"
        );
    }
}

#[test]
fn rejects_a_rule_without_readme() {
    let mut parts = rule("structure", "file-size-guard", "BTSTRUC001");
    parts.retain(|f| !f.path.ends_with("README.md"));
    let message = issues(vec![vec![family("structure", "STRUC")], parts]);
    assert!(message.contains("README.md"), "{message}");
}

#[test]
fn every_implementation_needs_languages_and_extensions_need_a_runtime() {
    let no_languages = "[[implementations]]\nkind = \"native\"\nlanguages = []\n";
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule_with("structure", "one", "BTSTRUC001", "", no_languages),
    ]);
    assert!(message.contains("languages"), "{message}");

    let missing_key = "[[implementations]]\nkind = \"native\"\n";
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule_with("structure", "one", "BTSTRUC001", "", missing_key),
    ]);
    assert!(message.contains("languages"), "{message}");

    let extension_without_runtime =
        "[[implementations]]\nkind = \"extension\"\nlanguages = [\"python\"]\n";
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule_with(
            "structure",
            "one",
            "BTSTRUC001",
            "",
            extension_without_runtime,
        ),
    ]);
    assert!(message.contains("runtime"), "{message}");

    let native_with_runtime =
        "[[implementations]]\nkind = \"native\"\nruntime = \"python\"\nlanguages = [\"*\"]\n";
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule_with("structure", "one", "BTSTRUC001", "", native_with_runtime),
    ]);
    assert!(message.contains("runtime"), "{message}");
}

#[test]
fn a_rule_needs_at_least_one_implementation() {
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule_with("structure", "one", "BTSTRUC001", "", ""),
    ]);
    assert!(message.contains("implementation"), "{message}");
}

#[test]
fn status_can_differ_per_implementation_and_languages_are_the_union() {
    let two = "[[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"python\"]\nfile = \"python.yml\"\n\n\
               [[implementations]]\nkind = \"ast-grep\"\nlanguages = [\"typescript\", \"javascript\"]\nstatus = \"preview\"\nfile = \"typescript.yml\"\n";
    let loaded = catalog(vec![
        vec![family("logging", "LOG")],
        rule_with("logging", "log-no-print", "BTLOG001", "", two),
    ])
    .expect("a rule with one implementation per language loads");

    let rule = loaded.resolve("log-no-print").expect("rule exists");
    assert_eq!(rule.status_of(&rule.implementations[0]), Status::Stable);
    assert_eq!(rule.status_of(&rule.implementations[1]), Status::Preview);
    assert_eq!(rule.languages(), ["javascript", "python", "typescript"]);
}

#[test]
fn rejects_unknown_keys_and_reports_every_problem_at_once() {
    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule_with("structure", "one", "BTSTRUC001", "typo-key = 1", NATIVE_ALL),
        rule("structure", "two", "BTARCH002"),
    ]);
    assert!(message.contains("typo-key"), "{message}");
    assert!(message.contains("BTARCH002"), "{message}");
}

#[test]
fn ces_is_only_a_documentary_reference_and_must_look_like_one() {
    let loaded = catalog(vec![
        vec![family("structure", "STRUC")],
        rule_with(
            "structure",
            "one",
            "BTSTRUC001",
            "ces = \"CES-71\"",
            NATIVE_ALL,
        ),
    ])
    .expect("a CES reference is allowed");
    assert_eq!(
        loaded.resolve("one").and_then(|r| r.ces.as_deref()),
        Some("CES-71")
    );
    assert!(loaded.resolve("CES-71").is_none(), "a CES is not an ID");

    let message = issues(vec![
        vec![family("structure", "STRUC")],
        rule_with("structure", "one", "BTSTRUC001", "ces = \"71\"", NATIVE_ALL),
    ]);
    assert!(message.contains("ces"), "{message}");
}
