"""A delegating skill is inert without the skill it delegates to.

`engineering-pr-review` and `engineering-refactor` carry a procedure and cite rule
*ids*; `engineering-rules` owns the text those ids resolve to. The `skills` CLI has no
dependency concept, so installing a leaf alone succeeds and leaves every citation
dangling. These tests pin the declaration and the check that catches it.

Exercised through `run_checks`, the public seam, rather than the private helper.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from scaffolding.checks import run_checks
from scaffolding.skills import LOCAL_SKILLS, SKILL_DEPENDENCIES

if TYPE_CHECKING:
    from pathlib import Path

SKILLS_DIR = ".agents/skills"
CHECK = "skill dependencies installed"


def _install(root: Path, *names: str) -> None:
    for name in names:
        d = root / SKILLS_DIR / name
        d.mkdir(parents=True)
        (d / "SKILL.md").write_text(f"---\nname: {name}\n---\n", encoding="utf-8")


def _result(root: Path):
    return next(c for c in run_checks(root) if c.name == CHECK)


def test_every_declared_dependency_is_itself_a_shipped_skill(tmp_path: Path):
    # A dependency the baseline never installs would make the check unsatisfiable.
    for leaf, dep in SKILL_DEPENDENCIES.items():
        assert leaf in LOCAL_SKILLS, f"{leaf} declares a dependency but is not in LOCAL_SKILLS"
        assert dep in LOCAL_SKILLS, f"{leaf} depends on {dep}, which LOCAL_SKILLS never installs"


def test_dependency_precedes_its_dependents_in_the_baseline():
    # `skills add` installs in list order; seeding a leaf before its dependency would
    # leave the tree transiently broken and reads as an ordering accident either way.
    for leaf, dep in SKILL_DEPENDENCIES.items():
        assert LOCAL_SKILLS.index(dep) < LOCAL_SKILLS.index(leaf), (
            f"{dep} must be listed before {leaf} in LOCAL_SKILLS"
        )


def test_passes_when_no_dependent_skill_is_installed(tmp_path: Path):
    # A repo that wants none of them is not missing anything.
    _install(tmp_path, "journalist")
    assert _result(tmp_path).ok


@pytest.mark.parametrize("leaf", sorted(SKILL_DEPENDENCIES))
def test_fails_when_a_leaf_is_installed_without_its_dependency(tmp_path: Path, leaf: str):
    _install(tmp_path, leaf)
    result = _result(tmp_path)
    assert not result.ok
    assert leaf in result.detail
    assert SKILL_DEPENDENCIES[leaf] in result.detail


@pytest.mark.parametrize("leaf", sorted(SKILL_DEPENDENCIES))
def test_passes_when_both_are_installed(tmp_path: Path, leaf: str):
    _install(tmp_path, leaf, SKILL_DEPENDENCIES[leaf])
    assert _result(tmp_path).ok


def test_names_every_broken_dependent_not_just_the_first(tmp_path: Path):
    # Reporting one at a time turns a single fix into several install rounds.
    leaves = sorted(SKILL_DEPENDENCIES)
    _install(tmp_path, *leaves)
    result = _result(tmp_path)
    assert not result.ok
    for leaf in leaves:
        assert leaf in result.detail


def test_remediation_command_installs_the_missing_dependency(tmp_path: Path):
    _install(tmp_path, "engineering-pr-review")
    detail = _result(tmp_path).detail
    assert "npx skills add collectiveai-team/botica --skill engineering-rules" in detail
