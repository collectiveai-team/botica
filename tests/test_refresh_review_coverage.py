"""`scripts/refresh_review_coverage.py` writes the expected block of each stale role.

Driven through `refresh`, with the JSON shape of `botica review check --format json` and role
files in a temporary directory.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location(
    "refresh_review_coverage", ROOT / "scripts" / "refresh_review_coverage.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
rrc = importlib.util.module_from_spec(_SPEC)
sys.modules["refresh_review_coverage"] = rrc
_SPEC.loader.exec_module(rrc)

OLD = '<!-- botica:covered -->\n- job "x" in "ci.yml": blake3 old\n<!-- /botica:covered -->'
NEW = '<!-- botica:covered -->\n- job "x" in "ci.yml": blake3 new\n<!-- /botica:covered -->'


def _role(name: str, status: str, block: str | None = NEW) -> rrc.Role:
    return {
        "role": name,
        "definition": {"path": f".github/review/{name}/REVIEW.md"},
        "covered": {"status": status, "expected_block": block},
    }


def _write_role(root: Path, name: str, block: str = OLD) -> Path:
    path = root / ".github" / "review" / name / "REVIEW.md"
    path.parent.mkdir(parents=True)
    path.write_text(f"---\nrole: {name}\n---\n\nProse.\n\n{block}\n")
    return path


def test_a_stale_block_is_replaced_and_the_rest_of_the_file_is_kept(tmp_path: Path) -> None:
    stale = _write_role(tmp_path, "spec")
    current = _write_role(tmp_path, "refute", NEW)

    changed = rrc.refresh(
        rrc.Report(roles=[_role("spec", "stale"), _role("refute", "current")]), tmp_path
    )

    assert changed == ["spec"]
    assert stale.read_text() == f"---\nrole: spec\n---\n\nProse.\n\n{NEW}\n"
    assert current.read_text() == f"---\nrole: refute\n---\n\nProse.\n\n{NEW}\n"


def test_an_unverified_role_fails(tmp_path: Path) -> None:
    _write_role(tmp_path, "spec")

    with pytest.raises(SystemExit, match="spec is unverified"):
        rrc.refresh(rrc.Report(roles=[_role("spec", "unverified", None)]), tmp_path)


def test_the_pins_workflow_refreshes_the_blocks_in_its_commit_and_asks_for_the_merge() -> None:
    workflow = (ROOT / ".github/workflows/pins.yml").read_text()
    step = workflow.split("      - name: Open or update the pull request\n", 1)[1]

    check = step.index("target/debug/botica review check --diff HEAD --head HEAD")
    refresh = step.index("scripts/refresh_review_coverage.py")
    amend = step.index("git commit -q --amend --no-edit --all")
    push = step.index("git push --force")
    assert check < refresh < amend < push
    assert "cargo build --locked -p botica-cli" in workflow
    assert step.index("gh pr edit chore/pins --add-label automerge") > push
    merge = (ROOT / ".github/workflows/dependabot-automerge-run.yml").read_text()
    assert "grep -qx automerge" in merge


def test_a_role_file_with_no_block_fails(tmp_path: Path) -> None:
    _write_role(tmp_path, "spec", "no markers")

    with pytest.raises(SystemExit, match="has no coverage block"):
        rrc.refresh(rrc.Report(roles=[_role("spec", "stale")]), tmp_path)
