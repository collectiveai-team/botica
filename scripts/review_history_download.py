"""Download bounded, complete history artifacts from trusted workflow runs."""

from __future__ import annotations

import io
import json
import os
import re
import selectors
import subprocess
import sys
import time
import zipfile
import zlib
from dataclasses import asdict, dataclass
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import TypedDict

MEMBER_LIMIT = 50 * 1024 * 1024
DOWNLOAD_BUDGET = 2 * 1024 * 1024 * 1024


@dataclass
class Extraction:
    """The complete history members extracted from one artifact."""

    runs: int
    findings: int
    bytes: int


@dataclass
class _HistoryRun:
    id: int
    head_sha: str


class _HistoryArtifact(TypedDict):
    id: int
    size_in_bytes: int
    expired: bool
    name: str


@dataclass
class DownloadSummary:
    """Downloaded evidence and the workflow runs that could not contribute history."""

    status: str
    accounted_bytes: int
    missing_runs: list[int]
    unavailable_workflows: list[str]


def _members(archive: zipfile.ZipFile, name: str, required: bool, limit: int) -> bytes:
    candidates = [
        member
        for member in archive.infolist()
        if member.filename in {name, f".botica/history/{name}"}
    ]
    if len(candidates) > 1 or (required and not candidates):
        raise ValueError(f"Missing or ambiguous history member: {name}")
    if not candidates:
        return b""
    member = candidates[0]
    if member.file_size > limit or member.is_dir():
        raise ValueError(f"History member exceeds its bound: {name}")
    with archive.open(member) as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise ValueError(f"History member exceeds its bound: {name}")
    return data


def extract_history(
    data: bytes,
    destination: Path,
    member_limit: int = MEMBER_LIMIT,
    total_limit: int = 2 * MEMBER_LIMIT,
) -> Extraction:
    """Validate ZIP integrity and bounds before writing either history member."""
    if len(data) > MEMBER_LIMIT:
        raise ValueError("Compressed history artifact exceeds 50 MiB")
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as archive:
            if len(archive.infolist()) > 128:
                raise ValueError("History archive contains too many members")
            runs = _members(archive, "runs.jsonl", True, member_limit)
            findings = _members(archive, "findings.jsonl", False, member_limit)
            if len(runs) + len(findings) > total_limit:
                raise ValueError("Uncompressed history exceeds the remaining budget")
    except (OSError, zipfile.BadZipFile, RuntimeError, zlib.error, EOFError) as error:
        raise ValueError("History archive is corrupt or unreadable") from error
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "runs.jsonl").write_bytes(runs)
    (destination / "findings.jsonl").write_bytes(findings)
    return Extraction(
        runs=len(runs.splitlines()),
        findings=len(findings.splitlines()),
        bytes=len(runs) + len(findings),
    )


def _read_bounded(process: subprocess.Popen[bytes], limit: int) -> bytes:
    if process.stdout is None:
        raise ValueError("GitHub history process has no output stream")
    deadline = time.monotonic() + 90
    chunks = bytearray()
    with selectors.DefaultSelector() as selector:
        selector.register(process.stdout, selectors.EVENT_READ)
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not selector.select(timeout=remaining):
                raise ValueError("GitHub history download timed out")
            chunk = os.read(process.stdout.fileno(), min(65536, limit + 1 - len(chunks)))
            if not chunk:
                return bytes(chunks)
            chunks.extend(chunk)
            if len(chunks) > limit:
                raise ValueError("GitHub history download exceeded its bound")


def _gh_bytes(endpoint: str, limit: int) -> bytes:
    process = subprocess.Popen(
        ["gh", "api", endpoint],
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
    )
    if process.stdout is None:
        raise ValueError("GitHub history process has no output stream")
    try:
        data = _read_bounded(process, limit)
        if process.wait(timeout=90) != 0:
            raise ValueError("GitHub history request failed")
        return data
    finally:
        process.stdout.close()
        if process.poll() is None:
            process.kill()
            process.wait()


def _pages(endpoint: str, key: str):
    for page in range(1, 1001):
        separator = "&" if "?" in endpoint else "?"
        response = json.loads(
            _gh_bytes(f"{endpoint}{separator}per_page=100&page={page}", 8 * 1024 * 1024)
        )
        found = response[key]
        if not isinstance(found, list):
            raise ValueError("GitHub history page has no item list")
        yield from found
        if len(found) < 100:
            return
    raise ValueError("GitHub history pagination exceeded its bound; report is incomplete")


def _ancestor(sha: str) -> bool:
    result = subprocess.run(
        [
            "git",
            "--no-replace-objects",
            "-c",
            "core.hooksPath=/dev/null",
            "merge-base",
            "--is-ancestor",
            sha,
            "HEAD",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
        timeout=90,
    )
    if result.returncode not in {0, 1}:
        raise ValueError("Cannot verify history artifact ancestry")
    return result.returncode == 0


def _expected_artifacts(repo: str, run_id: int, workflow: str) -> set[str]:
    expected = {
        "ci.yml": {"botica-history-lint"},
        "scheduled-review.yml": {"botica-history-scheduled-lint"},
        "review.yml": {"review-evidence"},
        "review-shadow.yml": set(),
        "review-feedback.yml": {"review-feedback"},
    }[workflow]
    if workflow == "scheduled-review.yml":
        jobs = list(_pages(f"repos/{repo}/actions/runs/{run_id}/jobs", "jobs"))
        report = [job for job in jobs if job["name"] == "survivors and score"]
        if len(report) != 1 or report[0]["conclusion"] != "skipped":
            expected.add("botica-history-scheduled-mutants")
    if workflow == "review-shadow.yml":
        return _shadow_artifacts(repo, run_id)
    if workflow == "review.yml" and _ignored_role_run(repo, run_id):
        return set()
    return expected


def _ignored_role_run(repo: str, run_id: int) -> bool:
    jobs = list(_pages(f"repos/{repo}/actions/runs/{run_id}/jobs", "jobs"))
    names = {job["name"] for job in jobs}
    return {"plan", "review-gate"}.issubset(names) and all(
        job["conclusion"] == "skipped" for job in jobs
    )


def _shadow_artifacts(repo: str, run_id: int) -> set[str]:
    jobs = list(_pages(f"repos/{repo}/actions/runs/{run_id}/jobs", "jobs"))
    expected = {
        f"review-shadow-{match[1]}"
        for job in jobs
        if (match := re.fullmatch(r"classify \(([1-9][0-9]*)\)", job["name"]))
        and job["conclusion"] != "skipped"
    }
    intentionally_skipped = bool(jobs) and all(
        job["conclusion"] == "skipped"
        or (job["name"] == "resolve" and job["conclusion"] == "success")
        for job in jobs
    )
    if expected or intentionally_skipped:
        return expected
    return {"unresolved-shadow-matrix"}


def _selected_artifact(name: object, workflow: str) -> bool:
    if not isinstance(name, str):
        return False
    if re.fullmatch(r"botica-history-[A-Za-z0-9._-]+", name):
        return True
    if workflow == "review-shadow.yml":
        return bool(re.fullmatch(r"review-shadow-[1-9][0-9]*", name))
    return name == {"review.yml": "review-evidence", "review-feedback.yml": "review-feedback"}.get(
        workflow
    )


def _download_artifact(
    repo: str, artifact: _HistoryArtifact, directory: Path, remaining: int, workflow: str
) -> tuple[int, bool]:
    size = int(artifact["size_in_bytes"])
    if artifact["expired"] or size > MEMBER_LIMIT or size > remaining:
        raise ValueError("Expected history artifact is unavailable or past the download budget")
    artifact_id = int(artifact["id"])
    data = _gh_bytes(
        f"repos/{repo}/actions/artifacts/{artifact_id}/zip", min(MEMBER_LIMIT, remaining)
    )
    if len(data) > remaining:
        raise ValueError("Actual history downloads exceeded the budget")
    if workflow == "review.yml":
        with zipfile.ZipFile(io.BytesIO(data)) as zipped:
            if not any(
                member.filename in {"runs.jsonl", ".botica/history/runs.jsonl"}
                for member in zipped.infolist()
            ):
                return len(data), False
    extracted = extract_history(data, directory / str(artifact_id), total_limit=remaining)
    return max(len(data), extracted.bytes), True


def _download_run(
    repo: str, run: _HistoryRun, directory: Path, remaining: int, workflow: str
) -> tuple[int, bool]:
    run_id = run.id
    expected = _expected_artifacts(repo, run_id, workflow)
    consumed = 0
    kept = set()
    for artifact in _pages(f"repos/{repo}/actions/runs/{run_id}/artifacts", "artifacts"):
        if not _selected_artifact(artifact["name"], workflow):
            continue
        size, complete = _download_artifact(
            repo, artifact, directory / str(run_id), remaining - consumed, workflow
        )
        consumed += size
        if complete:
            kept.add(artifact["name"])
    if kept:
        (directory / str(run_id) / "source.json").write_text(
            json.dumps({"id": str(run_id), "head_sha": run.head_sha, "trusted": True}),
            encoding="utf-8",
        )
    return consumed, expected.issubset(kept)


def _trusted_runs(repo: str, workflow: str, branch: str, since: str):
    branch_filter = "" if workflow.startswith("review") else f"branch={branch}&"
    endpoint = (
        f"repos/{repo}/actions/workflows/{workflow}/runs?{branch_filter}created=%3E%3D{since}"
    )
    allowed = {
        "review.yml": {"pull_request_target"},
        "review-shadow.yml": {"workflow_run", "workflow_dispatch", "issues"},
        "review-feedback.yml": {"workflow_run", "workflow_dispatch"},
    }.get(workflow, {"push", "schedule", "workflow_dispatch"})
    for run in _pages(endpoint, "workflow_runs"):
        if run["event"] not in allowed or run["head_repository"]["full_name"] != repo:
            continue
        sha = run["head_sha"]
        if not isinstance(sha, str) or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", sha):
            raise ValueError("Invalid workflow history SHA")
        if _ancestor(sha):
            yield _HistoryRun(int(run["id"]), sha)


def download_history(repo: str, branch: str, days: int, directory: Path) -> DownloadSummary:
    """Download all selected history artifacts or fail before a report can be published."""
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repo) or branch != "main" or days < 1:
        raise ValueError("Invalid repository, history window or destination")
    since = (datetime.now(UTC) - timedelta(days=days)).date().isoformat()
    remaining = DOWNLOAD_BUDGET
    missing = []
    unavailable = []
    available = {
        Path(workflow["path"]).name
        for workflow in _pages(f"repos/{repo}/actions/workflows", "workflows")
    }
    for workflow in [
        "ci.yml",
        "scheduled-review.yml",
        "review.yml",
        "review-shadow.yml",
        "review-feedback.yml",
    ]:
        if workflow not in available:
            unavailable.append(workflow)
            continue
        for run in _trusted_runs(repo, workflow, branch, since):
            consumed, complete = _download_run(repo, run, directory, remaining, workflow)
            remaining -= consumed
            if not complete:
                missing.append(run.id)
    return DownloadSummary(
        status="incomplete" if missing or unavailable else "complete",
        accounted_bytes=DOWNLOAD_BUDGET - remaining,
        missing_runs=missing,
        unavailable_workflows=unavailable,
    )


if __name__ == "__main__":
    request = json.load(sys.stdin)
    summary = download_history(
        request["repository"], request["branch"], int(request["days"]), Path(request["directory"])
    )
    print(json.dumps(asdict(summary)))
