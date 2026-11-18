"""The trusted workflow selects real review evidence instead of an ignored label event."""

import json
import os
import subprocess
from pathlib import Path

import pytest


@pytest.mark.parametrize("ignored", [True, False])
def test_shadow_ignores_a_skipped_run_but_not_a_pending_review(tmp_path: Path, ignored: bool):
    workflow = (
        Path(__file__).resolve().parents[1] / ".github/workflows/review-shadow.yml"
    ).read_text()
    body = workflow.split("- name: Read only the latest associated role evidence\n", 1)[1]
    body = body.split("        run: |\n", 1)[1].split("\n      - name:", 1)[0]
    script = "\n".join(line[10:] for line in body.splitlines())
    (tmp_path / ".tmp/results").mkdir(parents=True)
    runs = [_role_run(number, ignored) for number in [77, 78]]
    (tmp_path / "runs.json").write_text(json.dumps({"workflow_runs": runs}))
    for number in [77, 78]:
        state = "skipped" if number == 78 and ignored else "success"
        (tmp_path / f"jobs-{number}.json").write_text(
            json.dumps(
                {
                    "total_count": 2,
                    "jobs": [
                        {"name": name, "conclusion": state} for name in ["plan", "review-gate"]
                    ],
                }
            )
        )
    binary = tmp_path / "bin"
    binary.mkdir()
    gh = binary / "gh"
    gh.write_text(
        '#!/bin/sh\nif test "$1" = run; then\n'
        '  printf "%s\\n" "$3" >> "$DOWNLOADS"\n'
        "  mkdir -p .tmp/evidence/.tmp/results\n"
        '  echo "{}" > .tmp/evidence/.tmp/results/spec.json\n  exit 0\nfi\n'
        'for endpoint in "$@"; do :; done\n'
        'case "$endpoint" in\n'
        '  */runs/77/jobs*) cat "$ROOT/jobs-77.json" ;;\n'
        '  */runs/78/jobs*) cat "$ROOT/jobs-78.json" ;;\n'
        '  *) cat "$ROOT/runs.json" ;;\nesac\n',
    )
    gh.chmod(0o755)
    downloads = tmp_path / "downloads"
    result = subprocess.run(  # botica: ignore[test-no-subprocess] -- execute trusted download step
        ["bash", "-eo", "pipefail", "-c", script],
        cwd=tmp_path,
        env={
            **os.environ,
            "PATH": f"{binary}:{os.environ['PATH']}",
            "ROOT": str(tmp_path),
            "DOWNLOADS": str(downloads),
            "GH_REPO": "owner/repo",
            "PR_NUMBER": "42",
            "BASE_SHA": "b" * 40,
            "HEAD_SHA": "a" * 40,
        },
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    selected = json.loads((tmp_path / ".tmp/role-run.json").read_text())
    assert selected["id"] == (77 if ignored else 78)
    if ignored:
        assert downloads.read_text().strip() == "77"
        assert (tmp_path / ".tmp/results/spec.json").exists()
    else:
        assert not downloads.exists()
        assert not (tmp_path / ".tmp/results/spec.json").exists()


def _role_run(number: int, ignored: bool):
    completed = number == 77 or ignored
    return {
        "id": number,
        "event": "pull_request_target",
        "head_repository": {"full_name": "owner/repo"},
        "pull_requests": [
            {"number": 42, "head": {"sha": "a" * 40}, "base": {"sha": "b" * 40}},
        ],
        "status": "completed" if completed else "in_progress",
        "conclusion": "success" if completed else None,
    }
