use botica_catalog::{Catalog, Severity, SourceFile};
use botica_config::{Reason, Resolution};

mod common;

use common::{active, config, embedded, reason, resolve};

fn files_for(resolution: &Resolution, path: &str, language: Option<&str>) -> Vec<String> {
    let mut slugs: Vec<String> = resolution
        .rules_for(path, language)
        .iter()
        .map(|r| r.rule.id.clone())
        .collect();
    slugs.sort();
    slugs
}

struct R {
    slug: &'static str,
    code: &'static str,
    status: &'static str,
    default_enabled: bool,
    extra: &'static str,
    implementations: &'static str,
}

const ANY: &str = "[[implementations]]\nkind = \"native\"\nlanguages = [\"*\"]\n";

fn rule(slug: &'static str, code: &'static str) -> R {
    R {
        slug,
        code,
        status: "stable",
        default_enabled: true,
        extra: "",
        implementations: ANY,
    }
}

fn custom(rules: &[R]) -> Catalog {
    let mut files = vec![SourceFile {
        path: "structure/family.toml".into(),
        text: "code = \"STRUC\"\ndescription = \"d\"\n".into(),
    }];
    for r in rules {
        let text = format!(
            "id = \"{}\"\ncode = \"{}\"\nsummary = \"s\"\nstatus = \"{}\"\nsince = \"0.1.0\"\n{}\nscope = \"file\"\nneeds = \"path\"\n\
             run-on = [\"cli\"]\ndefault-enabled = {}\ndefault-severity = \"error\"\n{}",
            r.slug, r.code, r.status, r.extra, r.default_enabled, r.implementations
        );
        files.push(SourceFile {
            path: format!("structure/{}/rule.toml", r.slug),
            text,
        });
        files.push(SourceFile {
            path: format!("structure/{}/README.md", r.slug),
            text: "# r\n".into(),
        });
    }
    Catalog::from_files(&files).unwrap_or_else(|error| panic!("catalog:\n{error}"))
}

#[test]
fn ignore_is_reported_before_a_rules_own_switch() {
    let catalog = embedded();
    let resolution = resolve(
        &catalog,
        "ignore = [\"no-utils\"]\n[rules.no-utils]\nenabled = false\n",
    );
    assert_eq!(
        reason(&resolution, "no-utils"),
        Reason::IgnoredBy("no-utils".into())
    );
}

#[test]
fn a_rule_not_enabled_by_default_needs_all_or_an_explicit_selector() {
    let mut off = rule("opt-in", "BTSTRUC003");
    off.default_enabled = false;
    let catalog = custom(&[rule("always", "BTSTRUC001"), off]);
    assert_eq!(active(&resolve(&catalog, "")), ["always"]);
    assert_eq!(
        active(&resolve(&catalog, "select = [\"all\"]")),
        ["always", "opt-in"]
    );
    assert_eq!(
        active(&resolve(&catalog, "extend-select = [\"opt-in\"]")),
        ["always", "opt-in"]
    );
    assert_eq!(
        active(&resolve(&catalog, "extend-select = [\"BTSTRUC003\"]")),
        ["always", "opt-in"]
    );
}

#[test]
fn preview_rules_need_the_opt_in_even_when_selected_by_name() {
    let mut preview = rule("fresh", "BTSTRUC002");
    preview.status = "preview";
    let catalog = custom(&[rule("steady", "BTSTRUC001"), preview]);
    let resolution = resolve(&catalog, "extend-select = [\"fresh\"]");
    assert_eq!(active(&resolution), ["steady"]);
    assert_eq!(reason(&resolution, "fresh"), Reason::OnlyPreview);
    assert_eq!(
        active(&resolve(&catalog, "preview = true")),
        ["fresh", "steady"]
    );
}

#[test]
fn a_rule_with_a_preview_implementation_runs_the_stable_ones_only() {
    let two = "[[implementations]]\nkind = \"native\"\nlanguages = [\"python\"]\n\n\
               [[implementations]]\nkind = \"native\"\nlanguages = [\"typescript\"]\nstatus = \"preview\"\n";
    let mut mixed = rule("mixed", "BTSTRUC001");
    mixed.implementations = two;
    let catalog = custom(&[mixed]);
    let stable_only = resolve(&catalog, "");
    assert_eq!(files_for(&stable_only, "a.py", Some("python")), ["mixed"]);
    assert!(files_for(&stable_only, "a.ts", Some("typescript")).is_empty());
    let with_preview = resolve(&catalog, "preview = true");
    assert_eq!(
        files_for(&with_preview, "a.ts", Some("typescript")),
        ["mixed"]
    );

    let mut only_preview = rule("only", "BTSTRUC002");
    only_preview.implementations =
        "[[implementations]]\nkind = \"native\"\nlanguages = [\"*\"]\nstatus = \"preview\"\n";
    let catalog = custom(&[only_preview]);
    assert_eq!(reason(&resolve(&catalog, ""), "only"), Reason::OnlyPreview);
}

#[test]
fn a_removed_rule_is_never_active() {
    let mut gone = rule("gone", "BTSTRUC002");
    gone.status = "removed";
    let catalog = custom(&[rule("here", "BTSTRUC001"), gone]);
    let resolution = resolve(&catalog, "select = [\"all\"]");
    assert_eq!(active(&resolution), ["here"]);
    assert_eq!(reason(&resolution, "gone"), Reason::Removed);
}

#[test]
fn a_replacement_takes_the_place_of_an_active_rule_the_new_rule_declares_it_replaces() {
    let mut newer = rule("better", "BTSTRUC002");
    newer.default_enabled = false;
    newer.extra = "replaces = \"old\"";
    let catalog = custom(&[rule("old", "BTSTRUC001"), newer]);

    let resolution = resolve(&catalog, "[replace]\nold = \"better\"\n");
    assert_eq!(active(&resolution), ["better"]);
    assert_eq!(
        reason(&resolution, "old"),
        Reason::ReplacedBy("better".into())
    );
    assert_eq!(
        reason(&resolution, "better"),
        Reason::ReplacementOf("old".into())
    );

    let resolution = resolve(
        &catalog,
        "ignore = [\"old\"]\n[replace]\nold = \"better\"\n",
    );
    assert!(
        active(&resolution).is_empty(),
        "an ignored rule is not replaced"
    );

    let resolution = resolve(
        &catalog,
        "ignore = [\"better\"]\n[replace]\nold = \"better\"\n",
    );
    assert_eq!(
        reason(&resolution, "better"),
        Reason::IgnoredBy("better".into())
    );
}

#[test]
fn a_replacement_the_new_rule_does_not_declare_is_an_error() {
    let catalog = custom(&[rule("old", "BTSTRUC001"), rule("better", "BTSTRUC002")]);
    let error = Resolution::new(&config("[replace]\nold = \"better\"\n"), &catalog)
        .expect_err("not declared");
    assert!(error.to_string().contains("replaces"), "{error}");
}

#[test]
fn an_invalid_config_is_rejected_before_resolving() {
    let catalog = embedded();
    let error = Resolution::new(&config("select = [\"no-such-rule\"]"), &catalog)
        .expect_err("unknown rule");
    assert!(error.to_string().contains("no-such-rule"), "{error}");
}

#[test]
fn severity_and_options_are_the_rules_unless_the_config_changes_them() {
    let catalog = embedded();
    let resolution = resolve(&catalog, "");
    let entry = resolution.entry("file-size-guard").expect("rule");
    assert_eq!(entry.severity, Severity::Error);
    assert_eq!(entry.options["warn"].as_integer(), Some(400));
    assert_eq!(entry.options["max"].as_integer(), Some(700));

    let resolution = resolve(
        &catalog,
        "[rules.file-size-guard]\nseverity = \"warning\"\nwarn = 250\n",
    );
    let entry = resolution.entry("file-size-guard").expect("rule");
    assert_eq!(entry.severity, Severity::Warning);
    assert_eq!(entry.options["warn"].as_integer(), Some(250));
    assert_eq!(entry.options["max"].as_integer(), Some(700));
}

#[test]
fn a_rule_runs_only_on_the_languages_of_its_implementations() {
    let catalog = embedded();
    let resolution = resolve(&catalog, "");
    assert!(files_for(&resolution, "pkg/utils.py", Some("python")).contains(&"no-utils".to_owned()));
    assert!(!files_for(&resolution, "web/utils.ts", Some("typescript"))
        .contains(&"no-utils".to_owned()));
    assert!(files_for(&resolution, "web/a.ts", Some("typescript"))
        .contains(&"file-size-guard".to_owned()));
    assert!(files_for(&resolution, "notes.txt", None).contains(&"file-size-guard".to_owned()));
}

#[test]
fn the_languages_of_the_config_restrict_every_rule() {
    let catalog = embedded();
    let resolution = resolve(&catalog, "languages = [\"python\"]");
    assert!(!files_for(&resolution, "a.py", Some("python")).is_empty());
    assert!(files_for(&resolution, "a.ts", Some("typescript")).is_empty());
    assert!(files_for(&resolution, "notes.txt", None).is_empty());
}

#[test]
fn a_rules_default_ignores_apply_and_the_config_can_replace_or_extend_them() {
    let catalog = embedded();
    let size = |resolution: &Resolution, path: &str| {
        files_for(resolution, path, None).contains(&"file-size-guard".to_owned())
    };

    let defaults = resolve(&catalog, "");
    for skipped in [
        "Cargo.lock",
        "docs/guide.md",
        "a/b.svg",
        "x/fixtures/y/z.json",
        "app.min.js",
    ] {
        assert!(!size(&defaults, skipped), "{skipped} is ignored by default");
    }
    assert!(size(&defaults, "src/main.rs"));

    let cleared = resolve(&catalog, "[rules.file-size-guard]\nignores = []\n");
    assert!(size(&cleared, "Cargo.lock"));

    let replaced = resolve(
        &catalog,
        "[rules.file-size-guard]\nignores = [\"gen/**\"]\n",
    );
    assert!(size(&replaced, "Cargo.lock") && !size(&replaced, "gen/a.rs"));

    let extended = resolve(
        &catalog,
        "[rules.file-size-guard]\nextend-ignores = [\"gen/**\"]\n",
    );
    assert!(!size(&extended, "Cargo.lock") && !size(&extended, "gen/a.rs"));
}

#[test]
fn files_limits_where_a_rule_applies() {
    let mut scoped = rule("scoped", "BTSTRUC001");
    scoped.extra = "files = [\"src/**\"]";
    let catalog = custom(&[scoped]);
    let resolution = resolve(&catalog, "");
    assert_eq!(
        files_for(&resolution, "src/a.py", Some("python")),
        ["scoped"]
    );
    assert!(files_for(&resolution, "tests/a.py", Some("python")).is_empty());
}

#[test]
fn overrides_change_the_rules_of_the_files_they_match_in_order() {
    let catalog = embedded();
    let resolution = resolve(
        &catalog,
        "[[overrides]]\nfiles = [\"tests/**\"]\nignore = [\"no-utils\"]\nseverity = { file-size-guard = \"warning\" }\n\
         [[overrides]]\nfiles = [\"tests/legacy/**\"]\nseverity = { file-size-guard = \"off\" }\n",
    );
    assert!(files_for(&resolution, "src/utils.py", Some("python")).contains(&"no-utils".to_owned()));
    assert!(
        !files_for(&resolution, "tests/utils.py", Some("python")).contains(&"no-utils".to_owned())
    );

    let severity_of = |path: &str| {
        resolution
            .rules_for(path, None)
            .iter()
            .find(|r| r.rule.id == "file-size-guard")
            .map(|r| r.severity)
    };
    assert_eq!(severity_of("src/a.rs"), Some(Severity::Error));
    assert_eq!(severity_of("tests/a.rs"), Some(Severity::Warning));
    assert_eq!(
        severity_of("tests/legacy/a.rs"),
        None,
        "the later override turns it off"
    );
}

#[test]
fn external_ids_match_the_globs_of_the_configuration() {
    let catalog = embedded();
    let resolution = resolve(&catalog, "external = [\"acme-legacy/*\", \"old-rule\"]");
    assert!(resolution.is_external("acme-legacy/no-utils"));
    assert!(resolution.is_external("old-rule"));
    assert!(!resolution.is_external("no-utils"));
    assert!(!resolve(&catalog, "").is_external("acme-legacy/no-utils"));
}
