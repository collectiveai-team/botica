use super::*;

#[test]
fn trusted_custom_merge_driver_is_incomplete_and_never_executed() {
    let dir = repository("merge-driver", Some(DOCS));
    common::write(&dir, ".gitattributes", "docs/*.md merge=attacker\n");
    common::write(&dir, "docs/shared.md", "Initial\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Trusted driver"]);
    git(&dir, &["branch", "-f", "main", "HEAD"]);
    git(
        &dir,
        &["config", "merge.attacker.driver", "touch DRIVER_EXECUTED"],
    );
    common::write(&dir, "docs/shared.md", "Topic\n");
    git(&dir, &["commit", "-am", "Topic"]);
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["branch"]["status"], "complete");
    assert_eq!(
        report["integration"]["reasons"][0]["code"],
        "integration-failed"
    );
    assert!(!dir.join("DRIVER_EXECUTED").exists());
}

#[test]
fn trusted_attributes_control_size_despite_head_attributes_and_local_diff_commands() {
    let dir = repository("trusted-attributes", Some(DOCS));
    common::write(&dir, ".gitattributes", "docs/*.md -diff\n");
    common::write(&dir, "docs/shared.md", "Initial\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Trusted attributes"]);
    git(&dir, &["branch", "-f", "main", "HEAD"]);
    git(&dir, &["config", "diff.external", "touch DIFF_EXECUTED"]);
    common::write(&dir, ".gitattributes", "docs/*.md diff\n");
    common::write(&dir, "docs/shared.md", "Updated\n");
    git(&dir, &["commit", "-am", "Topic attributes"]);
    let (_, report) = classify(&dir, "merge");
    assert!(!dir.join("DIFF_EXECUTED").exists());
    let comparison = &report["branch"]["comparisons"][0];
    assert!(comparison["size"]["non_text_files"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("docs/shared.md")));
    assert_eq!(comparison["evidence"][1]["kind"], "binary");
}

#[cfg(unix)]
#[test]
fn linked_or_non_utf8_configuration_never_falls_back_to_defaults() {
    for linked in [true, false] {
        let dir = repository(
            if linked {
                "linked-config"
            } else {
                "invalid-config-encoding"
            },
            None,
        );
        if linked {
            std::os::unix::fs::symlink("docs/settings.toml", dir.join("botica.toml")).unwrap();
            common::write(&dir, "docs/settings.toml", DOCS);
        } else {
            common::write_bytes(&dir, "botica.toml", b"\xff");
        }
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-m", "Trusted configuration"]);
        git(&dir, &["branch", "-f", "main", "HEAD"]);
        let (output, report) = classify(&dir, "merge");
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(report["inputs"]["config"]["source"], "botica.toml");
        assert_eq!(report["branch"]["reasons"][0]["code"], "invalid-config");
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_git_paths_are_incomplete_instead_of_lossily_renamed() {
    use std::os::unix::ffi::OsStringExt;
    let dir = repository("non-utf8-path", Some(DOCS));
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(
        dir.join(std::ffi::OsString::from_vec(b"docs/\xff.md".to_vec())),
        "Note\n",
    )
    .unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Non UTF-8 path"]);
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["branch"]["reasons"][0]["code"], "unreadable-diff");
    assert!(report["effective_tier"].is_null());
}

#[test]
fn fastapi_route_declarations_are_facts_even_when_other_detector_support_is_missing() {
    let dir = repository("fastapi-route", None);
    common::write(&dir, "service.py", "from fastapi import FastAPI\napp = FastAPI()\n\n@app.get('/health')\ndef health():\n    return 'ok'\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Add endpoint"]);
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    let comparison = &report["branch"]["comparisons"][0];
    assert_eq!(comparison["signals"], serde_json::json!(["new-endpoint"]));
    assert!(comparison["signal_evidence"][0]["detail"]
        .as_str()
        .unwrap()
        .contains("/health"));
    assert_eq!(
        comparison["signal_evidence"][0]["detector"],
        "fastapi-decorators-v1"
    );
}

#[test]
fn changed_ci_migrations_deleted_tests_assertions_and_lowered_coverage_have_sources() {
    let dir = repository("review-facts", None);
    common::write(&dir, "tests/test_service.py", "assert secure()\n");
    common::write(
        &dir,
        "pyproject.toml",
        "[tool.coverage.report]\nfail_under = 90\n",
    );
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Baseline tests"]);
    git(&dir, &["branch", "-f", "main", "HEAD"]);
    std::fs::remove_file(dir.join("tests/test_service.py")).unwrap();
    common::write(
        &dir,
        "pyproject.toml",
        "[tool.coverage.report]\nfail_under = 40\n",
    );
    common::write(&dir, ".github/workflows/ci.yml", "name: CI\n");
    common::write(
        &dir,
        "db/migrations/001.sql",
        "CREATE TABLE events (id integer);\n",
    );
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-m", "Change review evidence"]);
    let (_, report) = classify(&dir, "merge");
    let comparison = &report["branch"]["comparisons"][0];
    for name in [
        "ci-changed",
        "migration-changed",
        "tests-deleted",
        "assertions-changed",
        "coverage-lowered",
    ] {
        assert!(
            comparison["signal_evidence"]
                .as_array()
                .unwrap()
                .iter()
                .any(|signal| signal["name"] == name
                    && !signal["paths"].as_array().unwrap().is_empty()),
            "{name}: {report}"
        );
    }
}

#[test]
fn submodules_cannot_be_hidden_by_local_ignore_configuration_or_generated_globs() {
    let config = format!("[review]\ngenerated = [\"/vendor/component.lock\"]\n{DOCS}\n[[review.areas]]\nname = \"periphery\"\npaths = [\"vendor/**\"]\n");
    let dir = repository("submodule", Some(&config));
    let old = git(&dir, &["rev-parse", "HEAD"]).trim().to_owned();
    git(
        &dir,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{old},vendor/component.lock"),
        ],
    );
    git(&dir, &["commit", "-m", "Initial gitlink"]);
    git(&dir, &["branch", "-f", "main", "HEAD"]);
    let new = git(&dir, &["rev-parse", "HEAD"]).trim().to_owned();
    git(&dir, &["config", "diff.ignoreSubmodules", "all"]);
    git(
        &dir,
        &["config", "submodule.vendor/component.lock.ignore", "all"],
    );
    git(
        &dir,
        &[
            "update-index",
            "--cacheinfo",
            &format!("160000,{new},vendor/component.lock"),
        ],
    );
    git(&dir, &["commit", "-m", "Update gitlink"]);
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    let comparison = &report["branch"]["comparisons"][0];
    assert_eq!(comparison["files"][0], "vendor/component.lock", "{report}");
    assert_eq!(comparison["evidence"][0]["kind"], "submodule");
    assert_eq!(comparison["evidence"][0]["generated"], true);
    assert!(report["effective_tier"].is_null());
}

#[test]
fn unreadable_trusted_config_blob_is_invalid_configuration_not_defaults() {
    let dir = repository("missing-config-object", Some(DOCS));
    let blob = git(&dir, &["rev-parse", "main:botica.toml"])
        .trim()
        .to_owned();
    std::fs::remove_file(dir.join(".git/objects").join(&blob[..2]).join(&blob[2..])).unwrap();
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["inputs"]["config"]["source"], "botica.toml");
    assert!(report["inputs"]["config"]["digest"].is_null());
    assert_eq!(report["branch"]["reasons"][0]["code"], "invalid-config");
}

#[test]
fn pyproject_configuration_is_read_from_destination_and_dual_sources_are_rejected() {
    for dual in [false, true] {
        let dir = repository(
            if dual {
                "dual-config"
            } else {
                "pyproject-config"
            },
            None,
        );
        let pyproject = DOCS.replace("[[review.areas]]", "[[tool.botica.review.areas]]");
        common::write(&dir, "pyproject.toml", &pyproject);
        if dual {
            common::write(&dir, "botica.toml", "");
        }
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-m", "Trusted config"]);
        git(&dir, &["branch", "-f", "main", "HEAD"]);
        common::write(&dir, "docs/note.md", "Note\n");
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-m", "Document"]);
        let (output, report) = classify(&dir, "merge");
        if dual {
            assert_eq!(output.status.code(), Some(2));
            assert_eq!(report["branch"]["reasons"][0]["code"], "invalid-config");
        } else {
            assert!(output.status.success(), "{}", common::stderr(&output));
            assert_eq!(report["effective_tier"], "low");
            assert_eq!(report["inputs"]["config"]["source"], "pyproject.toml");
            assert_eq!(
                report["inputs"]["config"]["digest"],
                blake3::hash(pyproject.as_bytes()).to_hex().to_string()
            );
        }
    }
}

#[test]
fn text_output_names_both_assessments_and_their_evidence() {
    let dir = repository("text-output", Some(DOCS));
    common::write(&dir, "docs/start.md", "Start\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Document"]);
    let head = git(&dir, &["rev-parse", "HEAD"]).trim().to_owned();
    let output = common::command(
        &dir,
        &[
            "review",
            "tier",
            "--diff",
            "refs/heads/main",
            "--integration-method",
            "merge",
            "--format",
            "text",
        ],
    );
    assert!(output.status.success(), "{}", common::stderr(&output));
    let text = common::stdout(&output);
    for fragment in [
        "effective risk: low",
        "branch risk: low",
        "integration risk: low",
        "review.areas[1]",
        "docs/start.md",
        "docs-only",
        "informational",
    ] {
        assert!(text.contains(fragment), "{fragment}: {text}");
    }
    assert!(text.contains(&head), "{text}");
}

#[test]
fn empty_merge_has_two_complete_medium_assessments_and_pinned_inputs() {
    let dir = common::project_in("review-tier", "empty-merge", None);
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Tier test"]);
    git(&dir, &["config", "user.email", "tier@example.com"]);
    git(&dir, &["commit", "--allow-empty", "-m", "Initial"]);
    let head = git(&dir, &["rev-parse", "HEAD"]).trim().to_owned();

    let output = common::command(
        &dir,
        &[
            "review",
            "tier",
            "--diff",
            "refs/heads/main",
            "--integration-method",
            "merge",
            "--format",
            "json",
        ],
    );

    assert!(output.status.success(), "{}", common::stderr(&output));
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("JSON classification");
    assert_eq!(report["status"], "complete");
    assert_eq!(report["inputs"]["base"], head);
    assert_eq!(report["inputs"]["head"], head);
    assert_eq!(report["inputs"]["integration_method"], "merge");
    assert_eq!(report["inputs"]["config"]["source"], "defaults");
    assert_eq!(report["effective_tier"], "medium");
    for assessment in ["branch", "integration"] {
        assert_eq!(report[assessment]["status"], "complete");
        assert_eq!(report[assessment]["tier"], "medium");
        assert_eq!(
            report[assessment]["comparisons"].as_array().unwrap().len(),
            1
        );
        assert_eq!(report[assessment]["comparisons"][0]["from"], head);
    }
}
