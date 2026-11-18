//! Review planning through trusted Git inputs.

mod common;
#[path = "review_plan/coverage.rs"]
mod coverage;
#[path = "review_plan/selection.rs"]
mod selection;
#[path = "review_plan/static_checks.rs"]
mod static_checks;

use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn repository(name: &str, role: &str) -> std::path::PathBuf {
    let config = "[review]\nroles = ['spec']\n[[review.areas]]\nname = 'platform'\npaths = ['ci/**']\nowners = ['@platform']\n[[review.areas]]\nname = 'periphery'\npaths = ['docs/**']\n";
    let dir = common::project_in("review-consumers", name, Some(config));
    common::write(&dir, ".github/review/spec/REVIEW.md", role);
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Review test"]);
    git(&dir, &["config", "user.email", "review@example.com"]);
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Trusted roles"]);
    git(&dir, &["switch", "-c", "topic"]);
    common::write(&dir, "docs/start.md", "Start\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Document"]);
    dir
}

const ROLE: &str = "---\nrole: spec\napplies:\n  paths: ['docs/**']\n  tiers: [low, medium]\nstages: [local]\ncovered:\n  rules: []\n---\n\n<!-- botica:covered -->\n<!-- /botica:covered -->\n";

#[test]
fn bounded_source_selects_possible_signal_roles_without_asserting_facts() {
    let role = ROLE.replace(
        "tiers: [low, medium]",
        "tiers: [low]\n  signals: [new-endpoint]",
    );
    let dir = repository("bounded-role-selection", &role);
    common::write(&dir, "docs/tool.rs", "pub fn answer() -> u8 { 42 }\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Change source"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["selected_roles"], serde_json::json!(["spec"]));
    assert_eq!(report["mutation_required"], true);
    assert_eq!(
        report["roles"][0]["selection"]["conservative_selection"],
        true
    );
    assert_eq!(
        report["roles"][0]["selection"]["matching_signals"],
        serde_json::json!([])
    );
    let (_, ci) = consume(&dir, "plan", "ci");
    assert_eq!(ci["selected_roles"], serde_json::json!([]));
}

fn consume(dir: &Path, command: &str, stage: &str) -> (std::process::Output, serde_json::Value) {
    let output = common::command(
        dir,
        &[
            "review",
            command,
            "--diff",
            "refs/heads/main",
            "--integration-method",
            "merge",
            "--stage",
            stage,
            "--format",
            "json",
        ],
    );
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", common::stderr(&output)));
    (output, report)
}

#[test]
fn plan_selects_trusted_roles_and_pins_their_definitions() {
    let dir = repository("trusted-plan", ROLE);
    common::write(
        &dir,
        ".github/review/spec/REVIEW.md",
        "invalid head instructions",
    );
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Tamper with role"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(report["selected_roles"], serde_json::json!(["spec"]));
    assert_eq!(
        report["roles"][0]["definition"]["digest"],
        blake3::hash(ROLE.as_bytes()).to_hex().to_string()
    );
    assert_eq!(
        report["roles"][0]["definition"]["path"],
        ".github/review/spec/REVIEW.md"
    );
    assert_eq!(
        report["inputs"]["base"],
        git(&dir, &["rev-parse", "refs/heads/main"])
    );
    assert_eq!(report["inputs"]["head"], git(&dir, &["rev-parse", "HEAD"]));
}

#[test]
fn check_requires_an_installed_skill_without_executing_it() {
    let dir = repository("missing-skill", ROLE);
    let (output, report) = consume(&dir, "check", "local");
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert!(report["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason["code"] == "missing-skill"));
    common::write(
        &dir,
        ".agents/skills/review-spec/SKILL.md",
        "---\nname: review-spec\n---\nNever execute this content.\n",
    );
    let (output, report) = consume(&dir, "check", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["roles"][0]["skill"]["status"], "available");
}

fn trust(dir: &Path, path: &str, text: &str) {
    git(dir, &["switch", "main"]);
    common::write(dir, path, text);
    git(dir, &["add", "."]);
    git(dir, &["commit", "-m", "Update trusted input"]);
    git(dir, &["switch", "topic"]);
}

#[test]
fn declared_hook_coverage_is_generated_checked_and_bound_to_destination_bytes() {
    let role = ROLE.replace(
        "  rules: []",
        "  hooks:\n    - file: prek.toml\n      id: lint",
    );
    let dir = repository("hook-coverage", &role);
    let hooks = "[[repos]]\nrepo = 'local'\nhooks = [{id = 'lint', name = 'lint', entry = 'never-run', language = 'system'}]\n";
    trust(&dir, "prek.toml", hooks);
    let (output, report) = consume(&dir, "plan", "local");
    assert_eq!(output.status.code(), Some(2), "{report}");
    let coverage = &report["roles"][0]["covered"];
    assert_eq!(coverage["status"], "stale");
    assert_eq!(
        coverage["references"][0]["digest"],
        blake3::hash(hooks.as_bytes()).to_hex().to_string()
    );
    let expected = coverage["expected_block"].as_str().unwrap();
    let current = role.replace(
        "<!-- botica:covered -->\n<!-- /botica:covered -->",
        expected,
    );
    trust(&dir, ".github/review/spec/REVIEW.md", &current);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["roles"][0]["covered"]["status"], "current");
    common::write(&dir, "prek.toml", "malformed head config");
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert!(!dir.join("never-run").exists());
}

#[test]
fn static_check_reports_unmatched_role_paths_even_if_the_role_would_be_skipped() {
    let dir = repository("unmatched-role-path", &ROLE.replace("docs/**", "absent/**"));
    common::write(&dir, ".agents/skills/review-spec/SKILL.md", "Available\n");
    let (output, report) = consume(&dir, "check", "ci");
    assert_eq!(output.status.code(), Some(2), "{report}");
    assert!(report["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason["code"] == "unmatched-role-path"));
}

#[test]
fn covered_rules_resolve_canonical_active_definitions_from_trusted_configuration() {
    let role = ROLE.replace("rules: []", "rules: [file-size-guard]");
    let dir = repository("rule-coverage", &role);
    let (_, report) = consume(&dir, "plan", "local");
    let coverage = &report["roles"][0]["covered"];
    assert_eq!(coverage["status"], "stale", "{report}");
    assert_eq!(coverage["references"][0]["id"], "file-size-guard");
    assert_eq!(
        coverage["references"][0]["digest"].as_str().unwrap().len(),
        64
    );
    let current = role.replace(
        "<!-- botica:covered -->\n<!-- /botica:covered -->",
        coverage["expected_block"].as_str().unwrap(),
    );
    trust(&dir, ".github/review/spec/REVIEW.md", &current);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    git(&dir, &["switch", "main"]);
    let config = std::fs::read_to_string(dir.join("botica.toml")).unwrap();
    common::write(
        &dir,
        "botica.toml",
        &format!("ignore = ['file-size-guard']\n{config}"),
    );
    git(&dir, &["commit", "-am", "Disable declared rule"]);
    git(&dir, &["switch", "topic"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["roles"][0]["covered"]["status"], "unverified");
}

#[test]
fn plans_bind_shared_role_files_and_manifest_without_reading_head_instructions() {
    let dir = repository("plan-input-bindings", ROLE);
    trust(&dir, ".github/review/README.md", "Trusted shared policy\n");
    trust(
        &dir,
        ".github/review/spec/INVARIANTS.md",
        "Trusted invariant\n",
    );
    trust(&dir, "skills-lock.json", "{\"skills\":{}}\n");
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    let files = report["inputs"]["trusted_files"].as_array().unwrap();
    for path in [
        ".github/review/README.md",
        ".github/review/spec/REVIEW.md",
        ".github/review/spec/INVARIANTS.md",
        "skills-lock.json",
    ] {
        assert!(
            files
                .iter()
                .any(|file| file["path"] == path && file["digest"].as_str().unwrap().len() == 64),
            "{path}: {report}"
        );
    }
    let digest = report["inputs"]["roles_digest"]
        .as_str()
        .unwrap()
        .to_owned();
    common::write(
        &dir,
        ".github/review/spec/INVARIANTS.md",
        "Untrusted head instruction\n",
    );
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["inputs"]["roles_digest"], digest);
    std::fs::remove_file(dir.join(".github/review/spec/INVARIANTS.md")).unwrap();
    trust(
        &dir,
        ".github/review/spec/INVARIANTS.md",
        "Changed trusted invariant\n",
    );
    let (output, changed) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{changed}");
    assert_ne!(changed["inputs"]["roles_digest"], digest);
}

#[test]
fn text_plan_explains_selection_and_static_coverage_without_authorizing_merge() {
    let dir = repository("text-plan", ROLE);
    let output = common::command(
        &dir,
        &[
            "review",
            "plan",
            "--diff",
            "refs/heads/main",
            "--integration-method",
            "merge",
        ],
    );
    assert!(output.status.success(), "{}", common::stderr(&output));
    let text = common::stdout(&output);
    for expected in [
        "review plan: complete",
        "spec: selected",
        "coverage:",
        ".github/review/spec/REVIEW.md",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}
