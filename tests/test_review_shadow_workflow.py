"""Execute the shadow workflow's effective-risk label publication with fake GitHub facts."""

import json
import os
import subprocess
from pathlib import Path

import pytest


@pytest.mark.parametrize(
    ("fresh_context", "prior", "current_tier", "measured"),
    [
        ("true", "", "high", False),
        ("false", "", "high", False),
        ("true", "high", "low", False),
        ("true", "high", "low", True),
    ],
)
def test_shadow_publishes_only_fresh_effective_risk_without_removing_area_labels(
    tmp_path: Path,
    fresh_context: str,
    prior: str,
    current_tier: str,
    measured: bool,
):
    workflow = (
        Path(__file__).resolve().parents[1] / ".github/workflows/review-shadow.yml"
    ).read_text()
    start = '          if test "$fresh_context" = true; then\n'
    script = start + workflow.split(start, 1)[1].split("\n          current=", 1)[0]
    script = "\n".join(line[10:] for line in script.splitlines())
    output = tmp_path / ".botica/review"
    output.mkdir(parents=True)
    (output / "handoff.json").write_text(
        json.dumps(
            {
                "effective_tier": current_tier,
                "probabilities": {"decision": 0.01} if measured else None,
            }
        )
    )
    (output / "handoff.md").write_text(
        "<!-- botica:review-handoff -->\n<!-- previous risk marker -->\nBlocked evidence.\n",
    )
    (tmp_path / ".tmp").mkdir()
    comments = tmp_path / "comments.json"
    comments.write_text(
        json.dumps(
            [
                {
                    "user": {"login": "github-actions[bot]"},
                    "body": f"<!-- botica:review-risk base={'b' * 40} "
                    f"head={'a' * 40} tier={prior} -->",
                }
            ]
        )
    )
    binary = tmp_path / "bin"
    binary.mkdir()
    gh = binary / "gh"
    gh.write_text(
        '#!/bin/sh\nif test "$1" = api; then cat "$COMMENTS";\n'
        'elif test "$1 $2" = "pr view"; then\n'
        '  printf "%s\\n" "risk:medium" "area:core"\n'
        'else\n  printf "%s\\n" "$*" >> "$GH_CALLS"\nfi\n',
    )
    gh.chmod(0o755)
    log = tmp_path / "github-calls"
    result = (
        subprocess.run(  # botica: ignore[test-no-subprocess] -- execute the public workflow seam
            ["bash", "-eo", "pipefail", "-c", script],
            cwd=tmp_path,
            env={
                **os.environ,
                "PATH": f"{binary}:{os.environ['PATH']}",
                "fresh_context": fresh_context,
                "PR_NUMBER": "42",
                "GH_CALLS": str(log),
                "COMMENTS": str(comments),
                "BASE_SHA": "b" * 40,
                "HEAD_SHA": "a" * 40,
            },
            capture_output=True,
            text=True,
            check=False,
        )
    )
    assert result.returncode == 0, result.stderr
    calls = log.read_text() if log.exists() else ""
    if fresh_context == "true":
        expected = current_tier if measured else prior or current_tier
        assert "pr edit 42 --remove-label risk:medium" in calls
        assert f"pr edit 42 --add-label risk:{expected}" in calls
        assert calls.index(f"label create risk:{expected}") < calls.index(
            f"pr edit 42 --add-label risk:{expected}"
        )
        assert "area:core" not in calls
        if prior and not measured:
            assert "Retained tier: high" in (output / "handoff.md").read_text()
            assert (
                json.loads((output / "handoff.json").read_text())["publication_risk"][
                    "effective_tier"
                ]
                == "high"
            )
    else:
        assert calls == ""


@pytest.mark.parametrize("changed", [True, False])
def test_publication_invalidates_its_labels_if_the_base_or_head_changes(
    tmp_path: Path, changed: bool
):
    workflow = (
        Path(__file__).resolve().parents[1] / ".github/workflows/review-shadow.yml"
    ).read_text()
    start = '          current=$(gh pr view "$PR_NUMBER"'
    script = start + workflow.rsplit(start, 1)[1].split("\n          comment=", 1)[0]
    script = "\n".join(line[10:] for line in script.splitlines())
    (tmp_path / ".botica/review").mkdir(parents=True)
    binary = tmp_path / "bin"
    binary.mkdir()
    gh = binary / "gh"
    gh.write_text(
        '#!/bin/sh\nif test "$1 $2" = "pr view"; then printf "%s\\n" "$CURRENT";\n'
        'else printf "%s\\n" "$*" >> "$GH_CALLS"; fi\n',
    )
    gh.chmod(0o755)
    log = tmp_path / "github-calls"
    result = (
        subprocess.run(  # botica: ignore[test-no-subprocess] -- public workflow with racing facts
            ["bash", "-eo", "pipefail", "-c", script],
            cwd=tmp_path,
            env={
                **os.environ,
                "PATH": f"{binary}:{os.environ['PATH']}",
                "PR_NUMBER": "42",
                "BASE_SHA": "b" * 40,
                "HEAD_SHA": "a" * 40,
                "GH_CALLS": str(log),
                "CURRENT": ("c" if changed else "b") * 40 + ":" + "a" * 40,
            },
            capture_output=True,
            text=True,
            check=False,
        )
    )
    assert result.returncode == 0, result.stderr
    calls = log.read_text() if log.exists() else ""
    if changed:
        assert "pr edit 42 --add-label risk:incomplete" in calls
        assert "pr edit 42 --remove-label risk:high" in calls
        assert "blocked" in (tmp_path / ".botica/review/handoff.md").read_text()
    else:
        assert calls == ""


@pytest.mark.parametrize(
    ("author", "head", "expected"),
    [
        ("github-actions[bot]", "a", "high"),
        ("github-actions[bot]", "c", "low"),
        ("other", "a", "low"),
    ],
)
def test_deterministic_labels_keep_a_pinned_classifier_increase(
    tmp_path: Path,
    author: str,
    head: str,
    expected: str,
):
    root = Path(__file__).resolve().parents[1]
    workflow = (root / ".github/workflows/review-labels.yml").read_text()
    start = "          published=$(gh api --paginate "
    script = start + workflow.split(start, 1)[1].split("\n          jq --argjson labels", 1)[0]
    script = "\n".join(line[10:] for line in script.splitlines())
    assert "group: review-publication-" in workflow
    assert (
        "group: review-publication-" in (root / ".github/workflows/review-shadow.yml").read_text()
    )
    (tmp_path / ".tmp").mkdir()
    (tmp_path / ".tmp/risk.json").write_text(
        json.dumps({"status": "complete", "effective_tier": "low"}),
    )
    comments = tmp_path / "comments.json"
    comments.write_text(
        json.dumps(
            [
                {
                    "user": {"login": author},
                    "body": "<!-- botica:review-handoff -->\n"
                    f"<!-- botica:review-risk base={'b' * 40} "
                    f"head={head * 40} tier=high -->\n",
                }
            ]
        )
    )
    binary = tmp_path / "bin"
    binary.mkdir()
    gh = binary / "gh"
    gh.write_text('#!/bin/sh\ncat "$COMMENTS"\n')
    gh.chmod(0o755)
    result = (
        subprocess.run(  # botica: ignore[test-no-subprocess] -- public publication ordering seam
            ["bash", "-eo", "pipefail", "-c", script],
            cwd=tmp_path,
            env={
                **os.environ,
                "PATH": f"{binary}:{os.environ['PATH']}",
                "PR_NUMBER": "42",
                "BASE_SHA": "b" * 40,
                "HEAD_SHA": "a" * 40,
                "GH_REPO": "owner/repo",
                "COMMENTS": str(comments),
            },
            capture_output=True,
            text=True,
            check=False,
        )
    )
    assert result.returncode == 0, result.stderr
    assert (
        json.loads((tmp_path / ".tmp/published-risk.json").read_text())["effective_tier"]
        == expected
    )
