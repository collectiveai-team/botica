use super::{common, fixture};
use std::process::Command;

#[test]
fn review_run_requires_the_selected_providers_credential() {
    let (dir, _) = fixture("run-provider-credential");
    for (flags, expected) in [
        (&[][..], "OPENAI_API_KEY is unavailable"),
        (
            &["--provider", "nvidia"][..],
            "NVIDIA_API_KEY is unavailable",
        ),
        (
            &["--provider", "nvidia", "--model", "nvidia/nvidia/old-id"][..],
            "NVIDIA NIM requires a publisher/model ID",
        ),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_botica"))
            .args([
                "review",
                "run",
                "--diff",
                "main",
                "--integration-method",
                "merge",
                "--context",
                ".tmp/context.json",
                "--role",
                "spec",
                "--output",
                ".tmp/new-results",
            ])
            .args(flags)
            .env_remove("OPENAI_API_KEY")
            .env_remove("NVIDIA_API_KEY")
            .current_dir(&dir)
            .output()
            .unwrap();
        let stderr = common::stderr(&result);
        assert_eq!(result.status.code(), Some(2), "{stderr}");
        assert!(stderr.contains(expected), "{stderr}");
    }
    assert!(!dir.join(".tmp/new-results/spec.json").exists());
}
