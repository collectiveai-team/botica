"""History archive ingestion rejects partial evidence through its public interface."""

import importlib.util
import io
import json
import os
import subprocess
import sys
import zipfile
from pathlib import Path

import pytest

_SPEC = importlib.util.spec_from_file_location(
    "review_history_download",
    Path(__file__).resolve().parents[1] / "scripts" / "review_history_download.py",
)
assert _SPEC is not None
assert _SPEC.loader is not None
download = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = download
_SPEC.loader.exec_module(download)


def archive(entries: list[tuple[str, bytes]]) -> bytes:
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as zipped:
        for name, content in entries:
            zipped.writestr(name, content)
    return output.getvalue()


def test_ingestion_reads_only_history_members_and_allows_a_run_with_no_findings(tmp_path: Path):
    result = download.extract_history(
        archive([("runs.jsonl", b'{"run":1}\n'), ("../../escape", b"never extract")]),
        tmp_path,
    )
    assert result.runs == 1
    assert (tmp_path / "runs.jsonl").read_bytes() == b'{"run":1}\n'
    assert (tmp_path / "findings.jsonl").read_bytes() == b""
    assert not (tmp_path.parent / "escape").exists()


@pytest.mark.parametrize(
    "entries",
    [
        [("findings.jsonl", b"{}\n")],
        [("runs.jsonl", b"{}\n"), (".botica/history/runs.jsonl", b"{}\n")],
        [("runs.jsonl", b"x" * 1025)],
    ],
)
def test_missing_ambiguous_and_oversized_histories_are_not_trusted(
    tmp_path: Path, entries: list[tuple[str, bytes]]
):
    with pytest.raises(ValueError, match=r"history member|History member"):
        download.extract_history(archive(entries), tmp_path, member_limit=1024)
    assert not (tmp_path / "runs.jsonl").exists()


def test_corrupt_archive_is_rejected_without_partial_output(tmp_path: Path):
    with pytest.raises(ValueError, match="corrupt or unreadable"):
        download.extract_history(b"not a zip archive", tmp_path)
    assert not (tmp_path / "runs.jsonl").exists()


def test_compressed_and_uncompressed_members_share_the_total_budget(tmp_path: Path):
    with pytest.raises(ValueError, match="remaining budget"):
        download.extract_history(
            archive([("runs.jsonl", b"{}\n"), ("findings.jsonl", b"{}\n")]),
            tmp_path,
            member_limit=1024,
            total_limit=4,
        )
    assert not (tmp_path / "runs.jsonl").exists()


@pytest.mark.parametrize("missing_mutants", [True, False])
def test_scheduled_run_requires_every_non_skipped_history_artifact(
    tmp_path: Path,
    missing_mutants: bool,
):
    names = ["botica-history-scheduled-lint"]
    if not missing_mutants:
        names.append("botica-history-scheduled-mutants")
    summary = run_download(tmp_path, "scheduled-review.yml", names)
    assert summary["status"] == ("incomplete" if missing_mutants else "complete")
    assert summary["missing_runs"] == ([7] if missing_mutants else [])


def test_discarded_role_archive_still_consumes_the_download_budget(tmp_path: Path):
    summary = run_download(tmp_path, "review.yml", ["review-evidence"], history=False)
    assert summary["status"] == "incomplete"
    assert summary["accounted_bytes"] == len(archive([("spec.json", b"{}\n")]))


@pytest.mark.parametrize("missing_pr", [True, False])
def test_issue_triggered_shadow_requires_each_matrix_artifact(tmp_path: Path, missing_pr: bool):
    names = ["review-shadow-42"]
    if not missing_pr:
        names.append("review-shadow-43")
    summary = run_download(tmp_path, "review-shadow.yml", names)
    assert summary["status"] == ("incomplete" if missing_pr else "complete")
    assert summary["missing_runs"] == ([7] if missing_pr else [])


def test_intentionally_skipped_shadow_does_not_create_a_false_coverage_gap(tmp_path: Path):
    summary = run_download(tmp_path, "review-shadow.yml", [], skipped_shadow=True)
    assert summary["status"] == "complete"
    assert summary["missing_runs"] == []
    assert summary["accounted_bytes"] == 0


def run_download(
    root: Path,
    workflow: str,
    names: list[str],
    *,
    history: bool = True,
    skipped_shadow: bool = False,
):
    answers = root / "answers"
    answers.mkdir()
    workflows = [
        "ci.yml",
        "scheduled-review.yml",
        "review.yml",
        "review-shadow.yml",
        "review-feedback.yml",
    ]
    responses: dict[str, object] = {
        "repos_owner_repo_actions_workflows": {
            "workflows": [{"path": f".github/workflows/{name}"} for name in workflows],
        },
        "repos_owner_repo_actions_runs_7_jobs": {
            "jobs": [{"name": "survivors and score", "conclusion": "success"}],
        },
        "repos_owner_repo_actions_runs_7_artifacts": {
            "artifacts": [
                {"id": index, "name": name, "expired": False, "size_in_bytes": 100}
                for index, name in enumerate(names, 1)
            ],
        },
    }
    if workflow == "review-shadow.yml":
        responses["repos_owner_repo_actions_runs_7_jobs"] = {
            "jobs": [{"name": f"classify ({pr})", "conclusion": "success"} for pr in [42, 43]],
        }
        if skipped_shadow:
            responses["repos_owner_repo_actions_runs_7_jobs"] = {
                "jobs": [
                    {"name": "resolve", "conclusion": "skipped"},
                    {"name": "classify", "conclusion": "skipped"},
                ],
            }
    for name in workflows:
        responses[f"repos_owner_repo_actions_workflows_{name}_runs"] = {
            "workflow_runs": [
                {
                    "id": 7,
                    "head_sha": "a" * 40,
                    "head_repository": {"full_name": "owner/repo"},
                    "event": {
                        "review.yml": "pull_request_target",
                        "review-shadow.yml": "issues",
                    }.get(name, "schedule"),
                }
            ]
            if name == workflow
            else [],
        }
    for name, value in responses.items():
        (answers / name).write_text(json.dumps(value))
    for index in range(1, len(names) + 1):
        (answers / f"repos_owner_repo_actions_artifacts_{index}_zip").write_bytes(
            archive([("runs.jsonl" if history else "spec.json", b"{}\n")]),
        )
    binary = root / "bin"
    binary.mkdir()
    (binary / "gh").write_text(
        '#!/bin/sh\nendpoint=${2%%\\?*}\nkey=$(printf "%s" "$endpoint" | tr / _)\n'
        'cat "$ANSWERS/$key"\n',
    )
    (binary / "git").write_text("#!/bin/sh\nexit 0\n")
    for path in binary.iterdir():
        path.chmod(0o755)
    result = (
        subprocess.run(  # botica: ignore[test-no-subprocess] -- public downloader with fake clients
            [sys.executable, str(_SPEC.origin)],
            input=json.dumps(
                {
                    "repository": "owner/repo",
                    "branch": "main",
                    "days": 28,
                    "directory": str(root / "download"),
                }
            ),
            capture_output=True,
            text=True,
            check=True,
            env={**os.environ, "PATH": f"{binary}:{os.environ['PATH']}", "ANSWERS": str(answers)},
        )
    )
    return json.loads(result.stdout)
