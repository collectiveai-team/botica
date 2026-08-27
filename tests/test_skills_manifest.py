"""CES-107 skills manifest: two states, the unignore op, and the disk post-condition.

Exercised through the public seams — ``build_plan``/``apply`` for the component and
``plan_manifest_ops`` for the manifest logic — never through private helpers.

The lock fixture is a real ``skills@1.5.22`` payload, fields and all. Several tests
depend on that: the point of this design is that we read the file and never write
it, so the fields we do not model must still be there afterwards.
"""

from __future__ import annotations

import json
import subprocess
from dataclasses import asdict, dataclass
from typing import TYPE_CHECKING

import pytest

from scaffolding.checks import run_checks
from scaffolding.engine import apply, build_plan
from scaffolding.facts import detect
from scaffolding.plan import Decisions, Disposition, Op
from scaffolding.settings import Settings
from scaffolding.skills import (
    MANIFEST_FILE,
    SkillsPlanInput,
    house_baseline,
    installed_names,
    plan_manifest_ops,
    read_manifest,
)

if TYPE_CHECKING:
    from pathlib import Path

SKILLS_DIR = ".agents/skills"


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    return tmp_path


def _install_skill(root: Path, name: str) -> None:
    d = root / SKILLS_DIR / name
    d.mkdir(parents=True, exist_ok=True)
    (d / "SKILL.md").write_text(f"# {name}\n", encoding="utf-8")


@dataclass(frozen=True)
class LockEntry:
    """An entry as the `skills` CLI writes it — ref, skillPath, hash and all.

    Mirrors upstream's `LocalSkillLockEntry`. The fields scaffolding does not model
    are the point: tests assert they survive a planning pass untouched.
    """

    source: str = "mattpocock/skills"
    ref: str = "v1.2.3"
    sourceType: str = "github"
    skillPath: str = "skills/engineering/tdd/SKILL.md"
    computedHash: str = "614ac2e45fb0ec02f6ce422d26bd9aa4e33aa4867323f5a3a5c0c20b96f78ff4"


def _write_lock(root: Path, names: list[str], source: str = "mattpocock/skills") -> str:
    skills = {n: asdict(LockEntry(source=source)) for n in sorted(names)}
    body = json.dumps({"version": 1, "skills": skills}, indent=2) + "\n"
    (root / MANIFEST_FILE).write_text(body, encoding="utf-8")
    return body


def _plan(root: Path, **kw) -> list[Op]:
    return plan_manifest_ops(
        SkillsPlanInput(
            root=root,
            agent="opencode",
            skills_dir=SKILLS_DIR,
            manifest_ignored=kw.pop("manifest_ignored", False),
            **kw,
        )
    )


def _kinds(ops: list[Op]) -> set[str]:
    return {op.kind for op in ops}


def _as_plan(root: Path, ops: list[Op]):
    plan = build_plan(
        root, detect(root, probe_visibility=False), Settings(skip_skills=True, skip_varlock=True)
    )
    plan.ops = ops
    return plan


# --- state 1: no lock file ---------------------------------------------------
def test_a_repo_with_no_lock_is_seeded_with_the_house_baseline(repo: Path):
    ops = _plan(repo)
    assert all(op.disposition is Disposition.RUN for op in ops)
    promised = {name for op in ops for name in (op.expect_skills or [])}
    assert promised == {e.name for e in house_baseline() if e.source_type != "local"}


def test_seed_ops_declare_a_post_condition_against_the_tree(repo: Path):
    # Without this the engine cannot tell "installed nothing" from "installed fine".
    for op in _plan(repo):
        assert op.expect_skills
        assert op.expect_dir == SKILLS_DIR


def test_seeding_warns_before_replacing_a_skill_already_on_disk(repo: Path):
    """`skills add` overwrites the directory, and the tree is gitignored.

    A hand-edited skill sharing a baseline name would otherwise be lost with no
    diff to recover it from (CES-30).
    """
    _install_skill(repo, "tdd")  # in the house baseline
    warned = [op for op in _plan(repo) if op.disposition is Disposition.WARN]
    assert [op.target for op in warned] == [f"{SKILLS_DIR}/tdd"]
    assert "replaced by the seed" in warned[0].detail


def test_installed_skills_outside_the_baseline_are_surfaced_not_invented(repo: Path):
    """Provenance is unrecoverable from disk, so we refuse to fabricate a lock entry."""
    _install_skill(repo, "tdd")
    _install_skill(repo, "our-private-skill")
    ops = _plan(repo)
    assert "write" not in _kinds(ops)  # the CLI owns the lock; we never author one
    undeclared = [
        op
        for op in ops
        if op.disposition is Disposition.WARN and op.target.startswith(MANIFEST_FILE)
    ]
    assert [op.target for op in undeclared] == [f"{MANIFEST_FILE}: our-private-skill"]
    assert "npx skills add" in undeclared[0].detail


# --- state 2: lock file present ----------------------------------------------
def test_a_repo_with_a_lock_only_restores(repo: Path):
    _write_lock(repo, ["tdd"])
    ops = _plan(repo)
    assert [op.cmd for op in ops] == [["npx", "skills", "experimental_install"]]


def test_the_lock_is_the_source_of_truth_and_is_never_topped_up(repo: Path):
    """A skill the repo dropped is a decision, not a gap (CES-30).

    The previous design topped the declared set up to the house baseline on every
    install, which made the baseline non-overridable and contradicted the premise
    that the lock file is authoritative.
    """
    _write_lock(repo, ["tdd"])
    ops = _plan(repo)
    assert not any(op.kind == "run" and "add" in (op.cmd or []) for op in ops)
    promised = {name for op in ops for name in (op.expect_skills or [])}
    assert promised == {"tdd"}


def test_restore_asserts_every_declared_skill_landed(repo: Path):
    _write_lock(repo, ["tdd", "triage"])
    (op,) = _plan(repo)
    assert op.expect_skills == ["tdd", "triage"]
    assert op.expect_dir == SKILLS_DIR


def test_planning_never_rewrites_the_lock_file(repo: Path):
    """The regression this design exists to prevent.

    Re-serialising the file from a narrower model drops ``ref`` (the only version
    pin there is), ``skillPath`` (whose absence makes `update` refetch every skill
    in the source repo) and ``computedHash``.
    """
    before = _write_lock(repo, ["tdd"])
    _plan(repo)
    _plan(repo, manifest_ignored=True)
    after = (repo / MANIFEST_FILE).read_text(encoding="utf-8")
    assert after == before
    assert json.loads(after)["skills"]["tdd"]["ref"] == "v1.2.3"


def test_an_unreadable_lock_is_deferred_never_overwritten(repo: Path):
    (repo / MANIFEST_FILE).write_text("{not json", encoding="utf-8")
    _install_skill(repo, "tdd")
    ops = _plan(repo)
    assert [op.disposition for op in ops] == [Disposition.DEFER]
    assert "write" not in _kinds(ops)
    assert (repo / MANIFEST_FILE).read_text(encoding="utf-8") == "{not json"


# --- the unignore op ---------------------------------------------------------
def test_an_ignored_lock_emits_a_scoped_unignore_op(repo: Path):
    (repo / ".gitignore").write_text(f".env\n{MANIFEST_FILE}\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    ops = _plan(repo, manifest_ignored=True)
    unignore = [op for op in ops if op.kind == "unignore"]
    assert len(unignore) == 1
    assert unignore[0].content == MANIFEST_FILE


def test_declining_the_unignore_warns_instead_of_editing(repo: Path):
    (repo / ".gitignore").write_text(f"{MANIFEST_FILE}\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    ops = _plan(repo, manifest_ignored=True, unignore=False)
    assert "unignore" not in _kinds(ops)
    assert any(op.disposition is Disposition.WARN for op in ops)


def test_a_non_literal_ignore_pattern_warns_instead_of_silently_doing_nothing(repo: Path):
    """`git check-ignore` is satisfied by patterns a literal line removal cannot fix."""
    (repo / ".gitignore").write_text("*.json\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    ops = _plan(repo, manifest_ignored=True)
    assert "unignore" not in _kinds(ops)
    warned = [op for op in ops if op.disposition is Disposition.WARN]
    assert len(warned) == 1
    assert "by hand" in warned[0].detail


def test_unignore_removes_only_the_whitelisted_line(repo: Path):
    gi = repo / ".gitignore"
    gi.write_text(".env\nskills-lock.json\n.tmp/\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    ops = [op for op in _plan(repo, manifest_ignored=True) if op.kind == "unignore"]
    apply(_as_plan(repo, ops), repo)
    assert gi.read_text(encoding="utf-8") == ".env\n.tmp/\n"


def test_unignore_preserves_crlf_and_a_missing_final_newline(repo: Path):
    gi = repo / ".gitignore"
    gi.write_bytes(b".env\r\nskills-lock.json\r\n.tmp/")
    _write_lock(repo, ["tdd"])
    ops = [op for op in _plan(repo, manifest_ignored=True) if op.kind == "unignore"]
    apply(_as_plan(repo, ops), repo)
    # Removing one line must not rewrite the endings of every other line.
    assert gi.read_bytes() == b".env\r\n.tmp/"


def test_unignore_refuses_a_line_outside_the_whitelist(repo: Path):
    gi = repo / ".gitignore"
    gi.write_text(".env\n", encoding="utf-8")
    rogue = Op("skills", "unignore", ".gitignore", Disposition.ADD, path=str(gi), content=".env")
    with pytest.raises(ValueError, match="non-whitelisted"):
        apply(_as_plan(repo, [rogue]), repo)


def test_the_unignore_op_is_not_reported_as_a_clean_add(repo: Path):
    """The ADR promises you can enumerate every destructive edit. `clean_adds` is not it."""
    (repo / ".gitignore").write_text(f"{MANIFEST_FILE}\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    ops = [op for op in _plan(repo, manifest_ignored=True) if op.kind == "unignore"]
    report = _as_plan(repo, ops).report()
    assert [o.kind for o in report.edits] == ["unignore"]
    assert "unignore" not in {o.kind for o in report.clean_adds}


# --- the post-condition ------------------------------------------------------
def test_a_run_that_delivers_nothing_is_an_error(repo: Path):
    op = Op(
        "skills",
        "run",
        "seed",
        Disposition.RUN,
        cmd=["true"],
        expect_skills=["tdd"],
        expect_dir=SKILLS_DIR,
    )
    assert apply(_as_plan(repo, [op]), repo) == 1


def test_a_run_that_delivers_passes(repo: Path):
    _install_skill(repo, "tdd")
    op = Op(
        "skills",
        "run",
        "seed",
        Disposition.RUN,
        cmd=["true"],
        expect_skills=["tdd"],
        expect_dir=SKILLS_DIR,
    )
    assert apply(_as_plan(repo, [op]), repo) == 0


def test_the_post_condition_reads_the_tree_not_the_lock(repo: Path):
    """A lock that declares a skill which was never written must still fail.

    Asking the tool to confirm its own bookkeeping verifies nothing.
    """
    _write_lock(repo, ["tdd"])
    op = Op(
        "skills",
        "run",
        "restore",
        Disposition.RUN,
        cmd=["true"],
        expect_skills=["tdd"],
        expect_dir=SKILLS_DIR,
    )
    assert apply(_as_plan(repo, [op]), repo) == 1


# --- manifest parsing --------------------------------------------------------
def test_malformed_manifest_reads_as_absent(repo: Path):
    (repo / MANIFEST_FILE).write_text("{not json", encoding="utf-8")
    assert read_manifest(repo) is None


def test_manifest_reads_declared_names(repo: Path):
    _write_lock(repo, ["handoff", "tdd"])
    manifest = read_manifest(repo)
    assert manifest is not None
    assert manifest.names == frozenset({"handoff", "tdd"})


def test_installed_names_ignores_dirs_without_a_skill_file(repo: Path):
    _install_skill(repo, "tdd")
    (repo / SKILLS_DIR / "not-a-skill").mkdir(parents=True)
    assert installed_names(repo, SKILLS_DIR) == ["tdd"]


# --- component + check wiring ------------------------------------------------
def test_seeding_a_bare_repo_asks_nothing_about_skills(repo: Path):
    plan = build_plan(repo, detect(repo, probe_visibility=False), Settings(skip_varlock=True))
    assert not [d for d in plan.decisions if d.key.startswith("skills_")]


def test_a_repo_with_a_lock_is_asked_nothing_about_its_skill_set(repo: Path):
    """The only consent left is about editing .gitignore, not about which skills."""
    _write_lock(repo, ["tdd"])
    plan = build_plan(repo, detect(repo, probe_visibility=False), Settings(skip_varlock=True))
    assert not [d for d in plan.decisions if d.key.startswith("skills_")]


def test_the_unignore_is_the_one_thing_consent_is_asked_for(repo: Path):
    (repo / ".gitignore").write_text(f"{MANIFEST_FILE}\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    plan = build_plan(repo, detect(repo, probe_visibility=False), Settings(skip_varlock=True))
    assert [d.key for d in plan.decisions if d.key.startswith("skills_")] == ["skills_unignore"]


def test_declining_the_unignore_is_honoured_end_to_end(repo: Path):
    (repo / ".gitignore").write_text(f"{MANIFEST_FILE}\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    plan = build_plan(
        repo,
        detect(repo, probe_visibility=False),
        Settings(skip_varlock=True),
        decisions=Decisions(skills_unignore=False),
    )
    assert not [op for op in plan.ops if op.kind == "unignore"]


def test_check_flags_an_ignored_manifest(repo: Path):
    (repo / ".gitignore").write_text(f"{MANIFEST_FILE}\n", encoding="utf-8")
    _write_lock(repo, ["tdd"])
    _install_skill(repo, "tdd")
    failed = {r.name for r in run_checks(repo) if not r.ok}
    assert "skills manifest not ignored" in failed


def test_check_flags_a_declared_but_missing_skill(repo: Path):
    _write_lock(repo, ["tdd"])
    results = {r.name: r for r in run_checks(repo)}
    assert not results["declared skills installed"].ok
    assert "tdd" in results["declared skills installed"].detail


def test_check_flags_an_installed_but_undeclared_skill(repo: Path):
    """Check drift in both directions.

    ``.agents/skills`` is gitignored, so an undeclared skill exists on one machine
    and evaporates on a fresh clone.
    """
    _write_lock(repo, ["tdd"])
    _install_skill(repo, "tdd")
    _install_skill(repo, "smuggled")
    results = {r.name: r for r in run_checks(repo)}
    assert not results["installed skills declared"].ok
    assert "smuggled" in results["installed skills declared"].detail
