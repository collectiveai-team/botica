use super::*;

#[test]
fn large_docs_changes_report_informational_size_without_a_size_gate() {
    let dir = repository("large-docs", Some(DOCS));
    common::write(&dir, "docs/book.md", &"A line.\n".repeat(1500));
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Document"]);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(report["effective_tier"], "low");
    let comparison = &report["branch"]["comparisons"][0];
    assert_eq!(comparison["size"]["changed_lines"], 1500);
    assert_eq!(comparison["size"]["informational_only"], true);
    assert_eq!(comparison["signals"], serde_json::json!(["docs-only"]));
}

#[test]
fn unknown_dependency_and_endpoint_support_is_explicit_incomplete_evidence() {
    for path in [
        "tools/dev-requirements.txt",
        "tools/environment-dev.txt",
        "tools/custom-input.txt",
        "tools/mise.toml",
        "tools/docker-compose.yml",
        "tools/packages.config",
        "tools/deno.json",
        "tools/service.zig",
    ] {
        let config =
            format!("{DOCS}\n[[review.areas]]\nname = \"periphery\"\npaths = [\"tools/**\"]\n");
        let dir = repository(
            &format!("unknown-{}", path.replace(['/', '.'], "-")),
            Some(&config),
        );
        common::write(&dir, path, "unrecognized package or endpoint declaration\n");
        git(&dir, &["add", "."]);
        git(&dir, &["commit", "-m", "Change unknown input"]);
        let (output, report) = classify(&dir, "merge");
        assert_eq!(output.status.code(), Some(2), "{path}: {report}");
        assert!(report["effective_tier"].is_null());
        assert_eq!(
            report["branch"]["reasons"][0]["code"],
            "unsupported-detector"
        );
        let comparison = &report["branch"]["comparisons"][0];
        assert_eq!(comparison["files"][0], path);
        assert!(comparison["signals"].as_array().unwrap().is_empty());
        assert_eq!(comparison["detector_evidence"][0]["status"], "unsupported");
        assert_eq!(comparison["detector_evidence"][0]["paths"][0], path);
    }
}

#[test]
fn new_npm_dependency_is_a_traceable_fact_that_raises_the_floor() {
    let config =
        format!("{DOCS}\n[[review.areas]]\nname = \"periphery\"\npaths = [\"tools/**\"]\n");
    let dir = repository("npm-dependency", Some(&config));
    common::write(
        &dir,
        "tools/package.json",
        r#"{"dependencies":{"added-package":"1.0.0"}}"#,
    );
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Add package"]);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(report["effective_tier"], "medium");
    let comparison = &report["branch"]["comparisons"][0];
    assert_eq!(comparison["signals"], serde_json::json!(["new-dependency"]));
    assert_eq!(
        comparison["signal_evidence"][0]["paths"],
        serde_json::json!(["tools/package.json"])
    );
    assert_eq!(
        comparison["signal_evidence"][0]["detector"],
        "npm-manifest-v1"
    );
    assert!(comparison["signal_evidence"][0]["detail"]
        .as_str()
        .unwrap()
        .contains("added-package"));
}

#[test]
fn forced_platform_paths_survive_periphery_and_case_differences() {
    let dir = repository("forced-platform", Some(DOCS));
    for path in ["CODEOWNERS", "docs/CODEOWNERS", "docs/Agents.md"] {
        common::write(&dir, path, "Policy\n");
    }
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Change policy"]);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(report["effective_tier"], "medium");
    for file in report["branch"]["comparisons"][0]["evidence"]
        .as_array()
        .unwrap()
    {
        assert_eq!(file["generated"], false);
        assert!(file["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["location"] == "forced-platform" && entry["area"] == "platform"));
    }
}

#[test]
fn generated_globs_cannot_exempt_forced_paths_with_different_case() {
    let config = format!("[review]\ngenerated = [\"docs/Agents.md\"]\n{DOCS}");
    let dir = repository("generated-forced-case", Some(&config));
    let (output, report) = classify(&dir, "merge");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(report["branch"]["reasons"][0]["code"], "invalid-config");
}

#[test]
fn generated_files_are_excluded_only_from_informational_size() {
    let config = format!("[review]\ngenerated = [\"docs/generated.md\"]\n{DOCS}");
    let dir = repository("generated-size", Some(&config));
    common::write(&dir, "docs/generated.md", &"Generated.\n".repeat(500));
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "Generate documentation"]);
    let (output, report) = classify(&dir, "merge");
    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(report["effective_tier"], "low");
    let comparison = &report["branch"]["comparisons"][0];
    assert_eq!(comparison["size"]["changed_lines"], 0);
    assert_eq!(comparison["evidence"][0]["generated"], true);
    assert_eq!(comparison["files"][0], "docs/generated.md");
}
