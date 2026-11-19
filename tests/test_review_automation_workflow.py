"""The published automation workflow cannot activate even on a requested true input."""

import subprocess
from pathlib import Path


def test_automation_policy_stays_disabled_for_both_dispatch_inputs(tmp_path: Path):
    workflow = (
        Path(__file__).resolve().parents[1] / ".github/workflows/review-automation.yml"
    ).read_text()
    policy = workflow.split("        run: |\n", 1)[1].split("\n  merge:", 1)[0]
    script = "\n".join(line[10:] for line in policy.splitlines())
    for value in ("true", "false"):
        output = tmp_path / f"policy-{value}"
        result = (
            subprocess.run(  # botica: ignore[test-no-subprocess] -- public workflow policy seam
                ["bash", "-eo", "pipefail", "-c", script],
                env={"GITHUB_OUTPUT": str(output), "REQUESTED_ENABLE": value},
                check=False,
                capture_output=True,
                text=True,
            )
        )
        assert result.returncode == 0, result.stderr
        assert output.read_text().strip() == "enabled=false"
    assert "needs.policy.outputs.enabled == 'true'" in workflow
    assert 'gh pr merge "$PR_NUMBER" --auto --merge --match-head-commit "$HEAD_SHA"' in workflow
