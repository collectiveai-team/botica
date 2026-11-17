use super::*;

#[test]
fn custom_catalog_definitions_are_resolved_without_running_their_extensions() {
    let role = ROLE.replace("rules: []", "rules: [acme/by-sh]");
    let dir = repository("trusted-custom-rule", &role);
    git(&dir, &["switch", "main"]);
    let config = std::fs::read_to_string(dir.join("botica.toml")).unwrap();
    common::write(
        &dir,
        "botica.toml",
        &format!("extend-select = ['acme/by-sh']\n{config}"),
    );
    let root = ".botica/rules/acme";
    common::write(&dir, &format!("{root}/catalog.toml"), "code = 'ACME'\n");
    common::write(
        &dir,
        &format!("{root}/structure/family.toml"),
        "code = 'STRUC'\ndescription = 'd'\n",
    );
    common::write(
        &dir,
        &format!("{root}/structure/by-sh/README.md"),
        common::project_catalog::README,
    );
    common::write(&dir, &format!("{root}/structure/by-sh/rule.toml"), "id = 'acme/by-sh'\ncode = 'ACMESTRUC001'\nsummary = 's'\nstatus = 'stable'\nscope = 'file'\nneeds = 'syntax'\nrun-on = ['cli']\ndefault-enabled = false\ndefault-severity = 'error'\n[[implementations]]\nkind = 'extension'\nruntime = 'command'\nlanguages = ['python']\ncommand = ['sh', '.botica/rules/acme/structure/by-sh/ext.sh']\n");
    common::write(
        &dir,
        &format!("{root}/structure/by-sh/ext.sh"),
        "touch SHOULD_NOT_EXECUTE\n",
    );
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Trusted extension definition"]);
    git(&dir, &["switch", "topic"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["roles"][0]["covered"]["status"], "stale", "{report}");
    assert_eq!(
        report["roles"][0]["covered"]["references"][0]["id"],
        "acme/by-sh"
    );
    assert!(!dir.join("SHOULD_NOT_EXECUTE").exists());
    let first = report["roles"][0]["covered"]["references"][0]["digest"].clone();
    trust(
        &dir,
        &format!("{root}/structure/by-sh/ext.sh"),
        "touch STILL_MUST_NOT_EXECUTE\n",
    );
    let (_, report) = consume(&dir, "plan", "local");
    assert_ne!(
        report["roles"][0]["covered"]["references"][0]["digest"],
        first
    );
    assert!(!dir.join("STILL_MUST_NOT_EXECUTE").exists());
}

#[test]
fn gitlab_role_discovery_requires_explicit_configuration_when_both_hosts_exist() {
    let dir = repository("gitlab-roles", ROLE);
    git(&dir, &["switch", "main"]);
    std::fs::create_dir_all(dir.join(".gitlab")).unwrap();
    git(&dir, &["mv", ".github/review", ".gitlab/review"]);
    git(&dir, &["commit", "-m", "GitLab role directory"]);
    git(&dir, &["switch", "topic"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["inputs"]["review_directory"], ".gitlab/review");
    trust(&dir, ".github/review/spec/REVIEW.md", ROLE);
    let (output, report) = consume(&dir, "plan", "local");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["reasons"][0]["code"], "invalid-policy-sources");
    git(&dir, &["switch", "main"]);
    let config = std::fs::read_to_string(dir.join("botica.toml")).unwrap();
    common::write(
        &dir,
        "botica.toml",
        &config.replace("[review]", "[review]\ndir = '.gitlab/review'"),
    );
    git(&dir, &["commit", "-am", "Choose trusted role directory"]);
    git(&dir, &["switch", "topic"]);
    let (output, report) = consume(&dir, "plan", "local");
    assert!(output.status.success(), "{report}");
    assert_eq!(report["selected_roles"], serde_json::json!(["spec"]));
}
