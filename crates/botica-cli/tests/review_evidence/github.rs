use super::*;
use std::os::unix::fs::PermissionsExt;

#[test]
fn classification_blocks_nonminimum_failed_pending_and_status_checks() {
    let (dir, plan) = configured_fixture(
        "github-checks",
        "[review]\nroles=['spec','refute']\n[review.classifier]\nrequired-checks=['ci']\n",
    );
    let check_head = "c".repeat(40);
    common::write(
        &dir,
        ".tmp/pull.json",
        &json!({"state":"open","draft":false,
        "base":{"sha":plan["inputs"]["base"]},"head":{"sha":plan["inputs"]["head"]},
        "merge_commit_sha":check_head})
        .to_string(),
    );
    common::write(
        &dir,
        ".tmp/commit.json",
        &json!({"tree":{"sha":plan["risk"]["integration"]["comparisons"][0]["to"]}}).to_string(),
    );
    common::write(
        &dir,
        ".tmp/role-runs.json",
        &json!({"workflow_runs":[{
            "id":77,"event":"pull_request_target","head_repository":{"full_name":"owner/repo"},
            "status":"completed","conclusion":"success","pull_requests":[{"number":42,
                "head":{"sha":plan["inputs"]["head"]},"base":{"sha":plan["inputs"]["base"]}}]
        }]})
        .to_string(),
    );
    common::write(
        &dir,
        ".tmp/artifacts.json",
        &json!({"total_count":1,"artifacts":[
            {"id":88,"name":"review-evidence","expired":false,"size_in_bytes":1000}
        ]})
        .to_string(),
    );
    for role in ["spec", "refute"] {
        common::write(
            &dir,
            &format!(".tmp/trusted-results/{role}.json"),
            &evidence_report(&dir, role).to_string(),
        );
    }
    common::write(
        &dir,
        ".tmp/bin/gh",
        r#"#!/bin/sh
if test "$1" = run; then
  while test "$#" -gt 0; do
    if test "$1" = --dir; then shift; directory=$1; fi
    shift
  done
  mkdir -p "$directory/.tmp/results"
  cp "$FAKE_GH_ROOT/.tmp/trusted-results/"*.json "$directory/.tmp/results/"
  exit 0
fi
case "$2" in
  */workflows/review.yml/runs*) cat "$FAKE_GH_ROOT/.tmp/role-runs.json" ;;
  */runs/77/artifacts*) cat "$FAKE_GH_ROOT/.tmp/artifacts.json" ;;
  */check-runs*) cat "$FAKE_GH_ROOT/.tmp/checks.json" ;;
  */status*) cat "$FAKE_GH_ROOT/.tmp/statuses.json" ;;
  */git/commits/*) cat "$FAKE_GH_ROOT/.tmp/commit.json" ;;
  *) cat "$FAKE_GH_ROOT/.tmp/pull.json" ;;
esac
"#,
    );
    std::fs::set_permissions(
        dir.join(".tmp/bin/gh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    for (status, conclusion, legacy, name, app, expected, tampered) in [
        (
            "completed",
            "success",
            "success",
            "other-test",
            "github-actions",
            "PASS",
            false,
        ),
        (
            "completed",
            "failure",
            "success",
            "other-test",
            "github-actions",
            "INCONCLUSIVE",
            false,
        ),
        (
            "in_progress",
            "",
            "success",
            "other-test",
            "github-actions",
            "INCONCLUSIVE",
            false,
        ),
        (
            "completed",
            "success",
            "pending",
            "other-test",
            "github-actions",
            "INCONCLUSIVE",
            false,
        ),
        (
            "completed",
            "success",
            "failure",
            "other-test",
            "github-actions",
            "INCONCLUSIVE",
            false,
        ),
        (
            "completed",
            "failure",
            "success",
            "botica-authorization",
            "external-app",
            "INCONCLUSIVE",
            false,
        ),
        (
            "completed",
            "failure",
            "success",
            "botica-authorization",
            "github-actions",
            "PASS",
            false,
        ),
        (
            "completed",
            "success",
            "success",
            "other-test",
            "github-actions",
            "PASS",
            true,
        ),
    ] {
        if tampered {
            let mut local = evidence_report(&dir, "spec");
            local["checked"] = json!(["forged extra coverage"]);
            common::write(&dir, ".tmp/results/spec.json", &local.to_string());
        }
        common::write(&dir, ".tmp/checks.json", &json!({"check_runs":[
            {"id":1,"name":"ci","head_sha":check_head,"status":"completed","conclusion":"success","app":{"slug":"github-actions"}},
            {"id":2,"name":name,"head_sha":check_head,"status":status,"conclusion":conclusion,"app":{"slug":app}}
        ]}).to_string());
        common::write(
            &dir,
            ".tmp/statuses.json",
            &json!({"statuses":[{"context":"external-test","state":legacy}]}).to_string(),
        );
        let output = Command::new(env!("CARGO_BIN_EXE_botica"))
            .args([
                "review",
                "classify",
                "--diff",
                "main",
                "--integration-method",
                "merge",
                "--context",
                ".tmp/context.json",
                "--results",
                ".tmp/results",
                "--repository",
                "owner/repo",
                "--github-pr",
                "42",
            ])
            .current_dir(&dir)
            .env("FAKE_GH_ROOT", &dir)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    dir.join(".tmp/bin").display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .env_remove("TYPESAFE_AI_API_KEY")
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", common::stderr(&output));
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["checks"]["producer_verified"], true, "{report}");
        assert_eq!(
            report["checks"]["ci"], expected,
            "{status}/{conclusion}/{legacy}"
        );
        assert_eq!(report["checks"]["jobs"]["ci"], "PASS");
        assert_eq!(
            report["review_summary"]["producer_verified"], !tampered,
            "{report}"
        );
        if tampered {
            assert_eq!(report["automation_eligible"], false);
            assert!(report["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|reason| reason["code"] == "review-evidence"));
        }
    }
}
