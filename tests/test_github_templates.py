"""GitHub templates install independently of CI without replacing local content."""

from __future__ import annotations

import subprocess
from typing import TYPE_CHECKING

import pytest

from scaffolding.engine import apply, build_plan
from scaffolding.facts import detect
from scaffolding.plan import Disposition
from scaffolding.settings import Settings

if TYPE_CHECKING:
    from pathlib import Path

TARGETS = (
    ".github/pull_request_template.md",
    ".github/ISSUE_TEMPLATE/bug_report.md",
    ".github/ISSUE_TEMPLATE/feature_proposal.md",
)


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    return tmp_path


def test_templates_are_default_on_without_ci(repo: Path):
    plan = build_plan(
        repo,
        detect(repo, probe_visibility=False),
        Settings(skip_skills=True, skip_varlock=True),
    )
    assert not any(op.component == "ci" for op in plan.ops)
    assert {op.target for op in plan.ops if op.component == "github-templates"} == set(TARGETS)


def test_explicit_install_adds_only_templates_and_is_idempotent(repo: Path):
    facts = detect(repo, probe_visibility=False)
    plan = build_plan(repo, facts, Settings(), requested=["github-templates"])
    assert {op.target for op in plan.ops} == set(TARGETS)
    assert all(op.disposition == Disposition.ADD for op in plan.ops)
    apply(plan, repo)
    assert "## Merge Danger" in (repo / TARGETS[0]).read_text()
    assert "## Measured evidence" in (repo / TARGETS[1]).read_text()
    assert "## Acceptance criteria" in (repo / TARGETS[2]).read_text()

    second = build_plan(repo, facts, Settings(), requested=["github-templates"])
    assert all(op.disposition == Disposition.DEFER for op in second.ops)
    assert not any(op.kind == "write" for op in second.ops)


@pytest.mark.parametrize("target", TARGETS)
def test_existing_template_is_preserved_while_missing_siblings_are_added(repo: Path, target: str):
    dest = repo / target
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_text("Local template with deliberate content\n")
    plan = build_plan(
        repo, detect(repo, probe_visibility=False), Settings(), requested=["github-templates"]
    )
    assert next(op for op in plan.ops if op.target == target).disposition == Disposition.DEFER
    apply(plan, repo)
    assert dest.read_text() == "Local template with deliberate content\n"
    assert all((repo / sibling).is_file() for sibling in TARGETS)


def test_templates_can_be_skipped(repo: Path):
    plan = build_plan(
        repo,
        detect(repo, probe_visibility=False),
        Settings(skip_skills=True, skip_varlock=True),
        skip=["github-templates"],
    )
    assert not any(op.component == "github-templates" for op in plan.ops)
