"""The schema accepts honest incomplete evidence for a critical Python revert."""

from __future__ import annotations

import json
import subprocess
from typing import TYPE_CHECKING

import pytest
from test_review_tier_schema import tier_binary as tier_binary
from test_review_tier_schema import validator

if TYPE_CHECKING:
    from pathlib import Path


@pytest.mark.integration
def test_criss_cross_python_revert_preserves_critical_evidence(tmp_path: Path, tier_binary: Path):
    def git(*args: str) -> str:
        result = subprocess.run(  # botica: ignore[test-no-subprocess] -- build CLI Git inputs
            ["git", *args], cwd=tmp_path, check=True, capture_output=True, text=True
        )
        return result.stdout.strip()

    def write(path: str, content: str):
        target = tmp_path / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")

    auth_path = "src/auth/check.py"
    old = "def authenticated(token):\n    return True\n"
    hardened = "def authenticated(token):\n    return token == 'trusted'\n"
    write(
        "botica.toml",
        """
[[review.areas]]
name = "platform"
paths = ["ci/**"]
owners = ["@platform"]
[[review.areas]]
name = "critical"
paths = ["src/auth/**"]
owners = ["@security"]
[[review.areas]]
name = "periphery"
paths = ["docs/**"]
""",
    )
    write(auth_path, old)
    git("init", "-b", "main")
    git("config", "user.name", "Schema test")
    git("config", "user.email", "schema@example.com")
    git("add", ".")
    git("commit", "-m", "Old authentication")
    root = git("rev-parse", "HEAD")
    write("docs/start.md", "Start\n")
    git("add", ".")
    git("commit", "-m", "P1 docs")
    p1 = git("rev-parse", "HEAD")
    old_tree = git("rev-parse", "HEAD^{tree}")
    git("switch", "--detach", root)
    write(auth_path, hardened)
    git("commit", "-am", "P2 authentication")
    p2 = git("rev-parse", "HEAD")
    write("docs/start.md", "Start\n")
    git("add", ".")
    hardened_tree = git("write-tree")
    destination = git("commit-tree", hardened_tree, "-p", p1, "-p", p2, "-m", "Destination")
    evil = git("commit-tree", old_tree, "-p", p1, "-p", p2, "-m", "Evil merge")
    git("branch", "-f", "main", destination)
    git("commit", "-m", "Prepare fixture tree")
    git("switch", "--detach", evil)
    write("docs/start.md", "Start updated\n")
    git("commit", "-am", "Topic documentation")
    assert set(git("merge-base", "--all", "refs/heads/main", "HEAD").splitlines()) == {p1, p2}

    output = subprocess.run(  # botica: ignore[test-no-subprocess] -- test the public CLI
        [
            str(tier_binary),
            "review",
            "tier",
            "--diff",
            "refs/heads/main",
            "--integration-method",
            "merge",
            "--format",
            "json",
        ],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        check=False,
    )
    assert output.returncode == 2, output.stderr
    report = json.loads(output.stdout)
    validator().validate(report)
    assert report["status"] == "incomplete"
    assert report["effective_tier"] is None
    comparisons = report["branch"]["comparisons"]
    assert len(comparisons) == 2
    assert any(c["from"] == p1 and c["tier"] == "low" for c in comparisons)
    critical = next(c for c in comparisons if c["from"] == p2)
    for name, comparison in [
        ("branch", critical),
        ("integration", report["integration"]["comparisons"][0]),
    ]:
        assert report[name]["status"] == "incomplete"
        assert "tier" not in report[name]
        assert report[name]["reasons"][0]["code"] == "unsupported-detector"
        assert comparison["tier"] == "high"
        assert "critical" in comparison["areas"]
        assert auth_path in comparison["files"]
        assert any(
            d["status"] == "unsupported" and d["paths"] == [auth_path]
            for d in comparison["detector_evidence"]
        )
        assert not {"new-dependency", "new-endpoint"} & set(comparison["signals"])
    tree = git("merge-tree", "--write-tree", "refs/heads/main", "HEAD")
    assert report["integration"]["comparisons"][0]["from"] == destination
    assert report["integration"]["comparisons"][0]["to"] == tree
    assert git("show", f"{tree}:{auth_path}") == old.strip()
    assert git("show", f"refs/heads/main:{auth_path}") == hardened.strip()
