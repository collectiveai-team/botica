use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use botica_conformance::Runner;

const RULE: &str = "my-rule";

const FINDING: &str = r#"{"rule":"my-rule","code":"BTX001","severity":"error","path":"bad.py","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":3,"line":1,"column":4}},"message":"bad thing"}"#;

const EXPECTED: &str = r#"{"diagnostics":[{"path":"bad.py","severity":"error","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":3,"line":1,"column":4}},"message":"bad thing"}]}"#;

const EXPECTED_ELSEWHERE: &str = r#"{"diagnostics":[{"path":"bad.py","severity":"error","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":3,"line":1,"column":4}},"message":"elsewhere"}]}"#;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create parents");
    fs::write(path, text).expect("write the file");
}

const OTHER_RULE_FINDING: &str = r#"{"rule":"other-rule","code":"BTX002","severity":"error","path":"other.py","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":1,"line":1,"column":2}},"message":"not mine"}"#;

fn finding_at(path: &str) -> String {
    FINDING.replace("bad.py", path)
}

/// A stand-in for `botica`: it reports `FINDING` on every project that holds a `bad.py` and a clean
/// run on the others, as a rule that dislikes files with that name would. A project that holds an
/// `other.py` also gets a finding of another rule.
fn build_fake_botica(dir: &Path) -> PathBuf {
    let script = r#"#!/bin/sh
found=''
add() {
  if [ -n "$found" ]; then found="$found,"; fi
  found="$found$1"
}
if [ -f bad.py ]; then add '@FINDING@'; fi
if [ -f other.py ]; then add '@OTHER@'; fi
if [ -f whole-file.py ]; then add '@WHOLE@'; fi
if [ -f crash.py ]; then echo 'boom' >&2; exit 2; fi
if [ -f cli-args.txt ]; then
  case " $* " in
    *" . "*) add '@WALKED@' ;;
  esac
fi
case " $* " in
  *" missing.py "*) add '@MISSING@' ;;
esac
for name in no-prefilter no-regions no-subtree-skip; do
  case " $* " in
    *" --$name "*)
      if [ -f "bug-$name" ]; then
        add "$(printf '{"rule":"my-rule","code":"BTX001","severity":"error","path":"bug-%s","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":1,"line":1,"column":2}},"message":"variant bug"}' "$name")"
      fi
      ;;
  esac
done
printf '%s' '{"schema_version":1,"diagnostics":['"$found"'],"errors":[]}'
[ -z "$found" ]
"#
    .replace("@FINDING@", FINDING)
    .replace("@OTHER@", OTHER_RULE_FINDING)
    .replace(
        "@WHOLE@",
        &finding_at("whole-file.py").replace(
            r#""range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":3,"line":1,"column":4}}"#,
            r#""range":null"#,
        ),
    )
    .replace("@WALKED@", &finding_at("walked"))
    .replace("@MISSING@", &finding_at("missing.py"));
    write(dir, "botica", &script);
    let path = dir.join("botica");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("make it executable");
    path
}

/// One script for every test, written once: writing and executing scripts from parallel tests
/// races on "Text file busy".
fn fake_botica() -> PathBuf {
    static FAKE: OnceLock<PathBuf> = OnceLock::new();
    FAKE.get_or_init(|| build_fake_botica(&scratch("fake_botica")))
        .clone()
}

struct Fixtures {
    valid: Vec<(&'static str, &'static str)>,
    invalid: Vec<(&'static str, &'static str)>,
    expected: Option<&'static str>,
}

fn good() -> Fixtures {
    Fixtures {
        valid: vec![("ok.py", "x = 1\n")],
        invalid: vec![("bad.py", "bad\n")],
        expected: Some(EXPECTED),
    }
}

fn lay_out(dir: &Path, fixtures: &Fixtures) -> PathBuf {
    let root = dir.join("fixtures");
    for (name, text) in &fixtures.valid {
        write(&root, &format!("valid/{name}"), text);
    }
    for (name, text) in &fixtures.invalid {
        write(&root, &format!("invalid/{name}"), text);
    }
    if let Some(text) = fixtures.expected {
        write(&root, "expected.json", text);
    }
    root
}

fn check(name: &str, fixtures: &Fixtures) -> botica_conformance::Outcome {
    let dir = scratch(name);
    let root = lay_out(&dir, fixtures);
    Runner::new(fake_botica(), dir.join("work"))
        .check(RULE, &root)
        .expect("the runner runs")
}

fn failures(outcome: &botica_conformance::Outcome) -> String {
    outcome
        .failures()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_rule_that_is_quiet_on_valid_and_reports_what_expected_lists_passes() {
    let outcome = check("passes", &good());
    assert!(outcome.passed(), "{}", failures(&outcome));
    assert_eq!(outcome.rule(), RULE);
}

#[test]
fn a_finding_in_a_valid_fixture_fails() {
    let mut fixtures = good();
    fixtures.valid.push(("bad.py", "bad\n"));
    let outcome = check("valid_finding", &fixtures);
    let text = failures(&outcome);
    assert!(!outcome.passed());
    assert!(
        text.contains("valid") && text.contains("bad.py") && text.contains("bad thing"),
        "{text}"
    );
}

#[test]
fn invalid_fixtures_that_raise_something_else_than_expected_fail() {
    let mut fixtures = good();
    fixtures.expected = Some(EXPECTED_ELSEWHERE);
    let outcome = check("mismatch", &fixtures);
    let text = failures(&outcome);
    assert!(
        text.contains("expected.json") && text.contains("elsewhere") && text.contains("bad thing"),
        "{text}"
    );
}

#[test]
fn findings_of_other_rules_are_not_this_rules_business() {
    let mut fixtures = good();
    fixtures.valid.push(("other.py", "x = 2\n"));
    fixtures.invalid.push(("other.py", "x = 2\n"));
    let outcome = check("other_rules", &fixtures);
    assert!(outcome.passed(), "{}", failures(&outcome));
}

#[test]
fn invalid_fixtures_without_expected_json_fail() {
    let mut fixtures = good();
    fixtures.expected = None;
    let text = failures(&check("no_expected", &fixtures));
    assert!(
        text.contains("expected.json") && text.contains("missing"),
        "{text}"
    );
}

#[test]
fn an_expected_json_that_lists_nothing_fails_because_invalid_must_be_invalid() {
    let mut fixtures = good();
    fixtures.expected = Some(r#"{"diagnostics":[]}"#);
    fixtures.invalid = vec![("fine.py", "x = 1\n")];
    let text = failures(&check("expects_nothing", &fixtures));
    assert!(
        text.contains("expected.json") && text.contains("nothing"),
        "{text}"
    );
}

#[test]
fn a_rule_needs_files_on_both_sides() {
    let mut fixtures = good();
    fixtures.valid.clear();
    let text = failures(&check("no_valid", &fixtures));
    assert!(
        text.contains("valid") && text.contains("no files"),
        "{text}"
    );

    let mut fixtures = good();
    fixtures.invalid.clear();
    let text = failures(&check("no_invalid", &fixtures));
    assert!(
        text.contains("invalid") && text.contains("no files"),
        "{text}"
    );
}

#[test]
fn every_variant_is_run_and_the_one_that_disagrees_is_named() {
    for variant in ["no-prefilter", "no-regions", "no-subtree-skip"] {
        let mut fixtures = good();
        fixtures.valid.push((
            Box::leak(format!("bug-{variant}").into_boxed_str()),
            "x = 3\n",
        ));
        let text = failures(&check(&format!("variant_{variant}"), &fixtures));
        assert!(
            text.contains(&format!("[{variant}]")) && text.contains("variant bug"),
            "{variant}: {text}"
        );
        for other in ["default", "no-prefilter", "no-regions", "no-subtree-skip"] {
            if other != variant {
                assert!(
                    !text.contains(&format!("[{other}]")),
                    "{variant} blamed {other}: {text}"
                );
            }
        }
    }
}

#[test]
fn cli_args_txt_replaces_the_walk_with_the_paths_it_lists() {
    let mut fixtures = good();
    fixtures.invalid = vec![("cli-args.txt", "missing.py\n")];
    fixtures.expected = Some(
        r#"{"diagnostics":[{"path":"missing.py","severity":"error","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":3,"line":1,"column":4}},"message":"bad thing"}]}"#,
    );
    let outcome = check("cli_args", &fixtures);
    assert!(outcome.passed(), "{}", failures(&outcome));
}

#[test]
fn a_botica_that_fails_is_an_error_of_the_run_not_a_verdict_on_the_rule() {
    let dir = scratch("crash");
    let mut fixtures = good();
    fixtures.valid.push(("crash.py", "x\n"));
    let root = lay_out(&dir, &fixtures);
    let error = Runner::new(fake_botica(), dir.join("work"))
        .check(RULE, &root)
        .expect_err("a failing botica stops the run");
    let text = format!("{error:#}");
    assert!(text.contains("boom"), "{text}");
}

#[test]
fn a_catalog_directory_is_checked_rule_by_rule_in_name_order() {
    let dir = scratch("catalog");
    let rules = dir.join("rules");
    write(&rules, "structure/family.toml", "code = \"STRUC\"\n");
    for (family, slug, fixtures) in [
        ("structure", "my-rule", good()),
        ("botica", "quiet-rule", good()),
    ] {
        let rule = rules.join(family).join(slug);
        write(&rule, "rule.toml", "id = \"x\"\n");
        lay_out(&rule, &fixtures);
    }
    write(
        &rules,
        "structure/not-a-rule/README.md",
        "# no rule.toml here\n",
    );

    let runner = Runner::new(fake_botica(), dir.join("work"));
    let outcomes = runner.check_catalog(&rules).expect("the catalog is read");

    let names: Vec<&str> = outcomes
        .iter()
        .map(botica_conformance::Outcome::rule)
        .collect();
    assert_eq!(names, ["my-rule", "quiet-rule"]);
    assert!(outcomes[0].passed());
    assert!(!outcomes[1].passed(), "the fake only ever reports my-rule");
}

#[test]
fn a_diagnostic_of_a_whole_file_has_no_range_and_is_compared_without_one() {
    let mut fixtures = good();
    fixtures.invalid = vec![("whole-file.py", "x = 1\n")];
    fixtures.expected = Some(
        r#"{"diagnostics":[{"path":"whole-file.py","severity":"error","range":null,"message":"bad thing"}]}"#,
    );
    let outcome = check("whole_file", &fixtures);
    assert!(outcome.passed(), "{}", failures(&outcome));

    fixtures.expected = Some(
        r#"{"diagnostics":[{"path":"whole-file.py","severity":"error","range":{"start":{"offset":0,"line":1,"column":1},"end":{"offset":3,"line":1,"column":4}},"message":"bad thing"}]}"#,
    );
    assert!(!check("whole_file_ranged", &fixtures).passed());
}
