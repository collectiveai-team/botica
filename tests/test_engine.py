"""Engine/plan tests: clean-adds-only behavior, gating, deps, idempotency."""

from __future__ import annotations

import dataclasses
import subprocess
from typing import TYPE_CHECKING

import pytest

from scaffolding.checks import run_checks
from scaffolding.components import AGENTS_MARKER, STANDARDS_MARKER
from scaffolding.engine import UnknownComponent, apply, build_plan, select_components
from scaffolding.facts import detect
from scaffolding.plan import Agent, Decisions, Disposition
from scaffolding.settings import Settings
from scaffolding.skills import LOCAL_SKILLS
from scaffolding.templates_registry import template_text

if TYPE_CHECKING:
    from pathlib import Path


def _git_init(path: Path) -> None:
    subprocess.run(["git", "init", "-q"], cwd=path, check=True)


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    _git_init(tmp_path)
    return tmp_path


def _facts(root: Path):
    return detect(root, probe_visibility=False)


def _targets(plan, disp: Disposition) -> list[str]:
    return [op.target for op in plan.by(disp)]


def test_nonpython_full_install_excludes_python_and_ci(repo: Path):
    plan = build_plan(repo, _facts(repo), Settings(skip_skills=True, skip_varlock=True))
    comps = {op.component for op in plan.ops}
    assert "ast-grep" not in comps
    assert "pyproject" not in comps
    assert "ci" not in comps
    assert ".gitignore" in _targets(plan, Disposition.ADD)
    assert "opencode.jsonc" in _targets(plan, Disposition.ADD)


def test_python_repo_includes_pyproject_and_ships_no_ast_grep_files(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    plan = build_plan(repo, _facts(repo), Settings(skip_skills=True, skip_varlock=True))
    targets = _targets(plan, Disposition.ADD)
    assert "pyproject.toml" in targets
    assert "sgconfig.yml" not in targets
    assert not [t for t in targets if t.startswith("ast-grep/")]
    keys = {d.key for d in plan.decisions}
    assert {"pyproject_name", "pyproject_description"} <= keys


def test_ast_grep_is_no_longer_a_component(repo: Path):
    with pytest.raises(UnknownComponent):
        build_plan(repo, _facts(repo), Settings(), requested=["ast-grep"])


def test_standards_pulls_agents_dependency(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    plan = build_plan(repo, _facts(repo), Settings(), requested=["standards"])
    comps = {op.component for op in plan.ops}
    assert "agents" in comps
    assert any("required by standards" in n for n in plan.notices)


def test_no_deps_suppresses_dependency(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    plan = build_plan(repo, _facts(repo), Settings(no_deps=True), requested=["standards"])
    comps = {op.component for op in plan.ops}
    assert "agents" not in comps


def test_explicit_selection_overrides_gate_with_notice(repo: Path):
    plan = build_plan(repo, _facts(repo), Settings(), requested=["pyproject"])
    assert any("gate not satisfied" in n for n in plan.notices)
    assert "pyproject.toml" in _targets(plan, Disposition.ADD)


def test_unknown_component_raises(repo: Path):
    with pytest.raises(UnknownComponent):
        build_plan(repo, _facts(repo), Settings(), requested=["nope"])


def test_apply_is_clean_adds_only_and_idempotent(repo: Path):
    settings = Settings(skip_skills=True, skip_varlock=True)
    plan = build_plan(repo, _facts(repo), settings)
    apply(plan, repo)
    assert (repo / ".gitignore").exists()
    assert (repo / "opencode.jsonc").exists()

    original = (repo / "opencode.jsonc").read_text()
    # Re-run: existing files must be deferred, not overwritten.
    plan2 = build_plan(repo, _facts(repo), settings)
    assert "opencode.jsonc" in _targets(plan2, Disposition.DEFER)
    apply(plan2, repo)
    assert (repo / "opencode.jsonc").read_text() == original


def test_pyproject_substitution(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    decisions = Decisions(pyproject_name="demo-pkg", pyproject_description="A demo")
    plan = build_plan(repo, _facts(repo), Settings(), requested=["pyproject"], decisions=decisions)
    apply(plan, repo)
    body = (repo / "pyproject.toml").read_text()
    assert 'name = "demo-pkg"' in body
    assert 'description = "A demo"' in body
    assert "replace-me" not in body


def test_gitignore_partial_append(repo: Path):
    (repo / ".gitignore").write_text(".env\n")
    plan = build_plan(repo, _facts(repo), Settings(), requested=["gitignore"])
    apply(plan, repo)
    lines = (repo / ".gitignore").read_text().splitlines()
    assert ".env" in lines
    assert ".journals/" in lines
    assert lines.count(".env") == 1


# test helper: a target->disposition lookup map is a legitimate dict boundary
def _standards_dispositions(  # botica: ignore[no-dict-return-annotation]
    plan,
) -> dict[str, Disposition]:
    return {op.target: op.disposition for op in plan.ops if op.component == "standards"}


def test_standards_clean_repo_adds_the_index_and_no_snippets_or_detail_files(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    plan = build_plan(repo, _facts(repo), Settings(), requested=["standards"])
    disp = _standards_dispositions(plan)
    adds = {t for t, d in disp.items() if d is Disposition.ADD}
    assert adds == {"AGENTS.md"}
    assert not [
        target for target in disp if target.startswith((".agents/rules/", ".agents/snippets/"))
    ]
    # the `agents` dependency is pulled in so AGENTS.md has a base to append to.
    assert any("required by standards" in n for n in plan.notices)


def test_standards_gated_out_on_nonpython(repo: Path):
    plan = build_plan(repo, _facts(repo), Settings(skip_skills=True, skip_varlock=True))
    comps = {op.component for op in plan.ops}
    assert "standards" not in comps


def test_standards_defers_and_skips_when_present(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    settings = Settings(skip_skills=True, skip_varlock=True)
    plan = build_plan(repo, _facts(repo), settings, requested=["standards"])
    apply(plan, repo)
    assert not (repo / ".agents/snippets").exists()
    body = (repo / "AGENTS.md").read_text()
    assert STANDARDS_MARKER in body
    assert AGENTS_MARKER in body

    plan2 = build_plan(repo, _facts(repo), settings, requested=["standards"])
    disp = _standards_dispositions(plan2)
    assert disp == {"AGENTS.md": Disposition.SKIP}

    # Re-apply must not duplicate the section or overwrite the workspace-defaults section.
    apply(plan2, repo)
    body2 = (repo / "AGENTS.md").read_text()
    assert body2.count(STANDARDS_MARKER) == 1
    assert AGENTS_MARKER in body2


def test_prek_plan_ships_repo_hygiene_through_the_botica_hook(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    plan = build_plan(repo, _facts(repo), Settings(), requested=["prek"])
    prek_ops = [op for op in plan.by(Disposition.ADD) if op.target == "prek.toml"]
    assert prek_ops, "expected a prek.toml ADD op"
    content = prek_ops[0].content or ""
    assert 'id = "botica"' in content
    for retired in ("file-size-guard", "no-utils", "repo-shape"):
        assert f'id = "{retired}"' not in content
    assert "{{ import_package }}" not in content


def test_prek_plan_ships_commit_msg_hook(repo: Path):
    plan = build_plan(repo, _facts(repo), Settings(), requested=["prek"])
    prek_ops = [op for op in plan.by(Disposition.ADD) if op.target == "prek.toml"]
    assert prek_ops, "expected a prek.toml ADD op"
    content = prek_ops[0].content or ""
    assert 'id = "agents-conventional-commits"' in content
    assert 'stages = ["commit-msg"]' in content


def test_ci_plan_ships_conventional_commits_workflow(repo: Path):
    plan = build_plan(repo, _facts(repo), Settings(), requested=["ci"])
    adds = {op.target for op in plan.by(Disposition.ADD) if op.component == "ci"}
    assert ".github/workflows/conventional-commits.yml" in adds


def test_conventional_commits_artifacts_are_ces_coded():
    prek = template_text("prek-generic.toml")
    assert "CES-75 (agents-conventional-commits)" in prek
    assert 'id = "agents-conventional-commits"' in prek
    wf = template_text("github/workflows/conventional-commits.yml")
    assert "CES-75 (agents-conventional-commits)" in wf
    # title read from env, never interpolated into the shell (injection-safe).
    assert "PR_TITLE: ${{ github.event.pull_request.title }}" in wf


def test_ci_plan_ships_commit_policy_workflow(repo: Path):
    # CES-91: commit-policy.yml ships unconditionally, mirroring conventional-commits.yml.
    plan = build_plan(repo, _facts(repo), Settings(), requested=["ci"])
    adds = {op.target for op in plan.by(Disposition.ADD) if op.component == "ci"}
    assert ".github/workflows/commit-policy.yml" in adds


def test_no_ai_coauthorship_artifacts_are_ces_coded():
    prek = template_text("prek-generic.toml")
    assert "CES-91 (no-ai-coauthorship)" in prek
    assert 'id = "no-ai-coauthorship"' in prek
    wf = template_text("github/workflows/commit-policy.yml")
    assert "CES-91 (no-ai-coauthorship)" in wf
    # base/head sha read from env, never interpolated into the shell (injection-safe).
    assert "BASE_SHA: ${{ github.event_name" in wf


def test_ci_plan_ships_security_workflows_for_python(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    decisions = Decisions(ci_parts=["security"])
    plan = build_plan(repo, _facts(repo), Settings(), requested=["ci"], decisions=decisions)
    adds = {op.target for op in plan.by(Disposition.ADD) if op.component == "ci"}
    assert ".github/workflows/dependency-review.yml" in adds
    # CES-119: osv-scanner replaces pip-audit — no pip-audit.yml target exists anymore.
    assert ".github/workflows/osv-scanner.yml" in adds
    assert ".github/workflows/pip-audit.yml" not in adds


def test_ci_plan_opencode_part_ships_the_opencode_workflow(repo: Path):
    decisions = Decisions(ci_parts=["opencode"])
    plan = build_plan(repo, _facts(repo), Settings(), requested=["ci"], decisions=decisions)
    adds = {op.target for op in plan.by(Disposition.ADD) if op.component == "ci"}
    assert ".github/workflows/opencode.yml" in adds


def test_ci_plan_skips_dependency_review_on_private_repos(repo: Path):
    # CES-113: tier-agnostic — dependency-review-action needs a paid GitHub Advanced
    # Security/Code Security tier on private/internal repos, so it's skipped there (same
    # posture as docker.yml's GHCR-billing skip), not silently shipped broken.
    decisions = Decisions(ci_parts=["security"])
    facts = dataclasses.replace(_facts(repo), visibility="private")
    plan = build_plan(repo, facts, Settings(), requested=["ci"], decisions=decisions)
    adds = {op.target for op in plan.by(Disposition.ADD) if op.component == "ci"}
    skips = {op.target for op in plan.by(Disposition.SKIP) if op.component == "ci"}
    assert ".github/workflows/dependency-review.yml" not in adds
    assert ".github/workflows/dependency-review.yml" in skips
    # zizmor (no tier requirement) still ships.
    assert ".github/workflows/zizmor.yml" in adds


def test_ci_plan_skips_osv_scanner_for_nonpython_security(repo: Path):
    # osv-scanner is currently gated on uv.lock (Python); non-Python repos get zizmor +
    # dependency-review only, until a non-Python template exists (see CES-119's detail file).
    decisions = Decisions(ci_parts=["security"])
    plan = build_plan(repo, _facts(repo), Settings(), requested=["ci"], decisions=decisions)
    adds = {op.target for op in plan.by(Disposition.ADD) if op.component == "ci"}
    assert ".github/workflows/dependency-review.yml" in adds
    assert ".github/workflows/osv-scanner.yml" not in adds


def test_dependency_review_and_osv_scanner_are_ces_coded():
    wf = template_text("github/workflows/dependency-review.yml")
    assert "CES-113 (dependency-review-action)" in wf
    assert "comment-summary-in-pr: always" in wf
    # the action hard-fails (not a quiet no-op) when Dependency graph isn't enabled — this must
    # never block a merge, since the whole point of the check is "summary-only".
    assert "continue-on-error: true" in wf
    wf2 = template_text("github/workflows/osv-scanner.yml")
    assert "CES-119 (osv-scanner-replace-pip-audit)" in wf2
    assert "--lockfile=uv.lock" in wf2


def test_prek_plan_ships_deptry_and_complexipy_hooks(repo: Path):
    (repo / "pyproject.toml").write_text("[project]\nname='x'\n")
    plan = build_plan(repo, _facts(repo), Settings(), requested=["prek"])
    prek_ops = [op for op in plan.by(Disposition.ADD) if op.target == "prek.toml"]
    assert prek_ops, "expected a prek.toml ADD op"
    content = prek_ops[0].content or ""
    assert 'id = "deptry"' in content
    assert 'id = "complexipy"' in content


def test_deptry_and_complexipy_are_ces_coded():
    prek = template_text("prek-python.toml")
    assert "CES-109" in prek
    assert 'id = "deptry"' in prek
    assert "CES-110" in prek
    assert 'id = "complexipy"' in prek


def test_prek_plan_ships_jscpd_and_hadolint_hooks(repo: Path):
    plan = build_plan(repo, _facts(repo), Settings(), requested=["prek"])
    prek_ops = [op for op in plan.by(Disposition.ADD) if op.target == "prek.toml"]
    assert prek_ops, "expected a prek.toml ADD op"
    content = prek_ops[0].content or ""
    assert 'id = "jscpd"' in content
    assert 'id = "hadolint-docker"' in content


def test_jscpd_and_hadolint_are_ces_coded():
    prek = template_text("prek-generic.toml")
    assert "CES-118" in prek
    assert 'id = "jscpd"' in prek
    assert "CES-114" in prek
    # file-pattern-scoped: inert without a Dockerfile, same posture as repo-shape.
    assert 'files = "^Dockerfile"' in prek


def test_pytest_randomly_ships_in_dev_group():
    body = template_text("pyproject-template.toml")
    assert "CES-111" in body
    assert "pytest-randomly" in body


def test_pyproject_plan_adds_on_clean_defers_on_existing(repo: Path):
    (repo / "app.py").write_text("x = 1\n")
    plan = build_plan(repo, _facts(repo), Settings(), requested=["pyproject"])
    assert "pyproject.toml" in _targets(plan, Disposition.ADD)
    apply(plan, repo)
    plan2 = build_plan(repo, _facts(repo), Settings(), requested=["pyproject"])
    assert "pyproject.toml" in _targets(plan2, Disposition.DEFER)


def test_pyproject_template_version_pin_and_importlinter_skeleton():
    body = template_text("pyproject-template.toml")
    # CES-77: requires-python is a non-enforced comment defaulting to 3.14.
    assert "CES-77 (version pin)" in body
    assert '# requires-python = ">=3.14"' in body
    # the active requires-python line is gone (commented only).
    active = [ln for ln in body.splitlines() if ln.strip().startswith("requires-python")]
    assert active == []
    # CES-5 / CES-17: commented import-linter skeleton present.
    assert "# [tool.importlinter]" in body
    assert "Layered architecture (CES-5)" in body
    assert "CES-17" in body


def test_judgment_tier_index_entries_marked_judgment():
    index = template_text("standards-index.md")
    for code in ("CES-16", "CES-8", "CES-30", "CES-58", "CES-64", "CES-65", "CES-66"):
        assert code in index


def test_the_index_points_at_no_detail_file():
    assert "@.agents/rules/" not in template_text("standards-index.md")


def test_select_skip(repo: Path):
    comps, _ = select_components([], ["agent-config"], _facts(repo), Settings(skip_skills=True))
    names = {c.name for c in comps}
    assert "agent-config" not in names
    assert "gitignore" in names


# --- multi-agent config ------------------------------------------------------
def _no_skills(**kw) -> Settings:
    return Settings(skip_skills=True, skip_varlock=True, **kw)


def test_default_agent_still_writes_opencode_jsonc(repo: Path):
    plan = build_plan(repo, _facts(repo), _no_skills())
    assert "opencode.jsonc" in _targets(plan, Disposition.ADD)


def test_claude_agent_writes_settings_and_symlinks_not_opencode(repo: Path):
    decisions = Decisions(agents=[Agent.CLAUDE_CODE])
    plan = build_plan(repo, _facts(repo), _no_skills(), decisions=decisions)
    adds = _targets(plan, Disposition.ADD)
    assert ".claude/settings.json" in adds
    assert "CLAUDE.md" in adds
    assert ".claude/skills" in adds
    assert "opencode.jsonc" not in adds


def test_claude_symlinks_apply_and_defer_on_rerun(repo: Path):
    decisions = Decisions(agents=[Agent.CLAUDE_CODE])
    settings = _no_skills()
    apply(build_plan(repo, _facts(repo), settings, decisions=decisions), repo)
    claude_md = repo / "CLAUDE.md"
    assert claude_md.is_symlink()
    assert claude_md.readlink().name == "AGENTS.md"
    assert (repo / ".claude" / "skills").is_symlink()
    assert claude_md.resolve() == (repo / "AGENTS.md").resolve()

    plan2 = build_plan(repo, _facts(repo), settings, decisions=decisions)
    defers = _targets(plan2, Disposition.DEFER)
    assert "CLAUDE.md" in defers
    assert ".claude/skills" in defers


def test_multi_agent_opencode_and_codex(repo: Path):
    decisions = Decisions(agents=[Agent.OPENCODE, Agent.CODEX])
    plan = build_plan(repo, _facts(repo), _no_skills(), decisions=decisions)
    assert "opencode.jsonc" in _targets(plan, Disposition.ADD)
    codex_notice = [
        op for op in plan.ops if op.component == "agent-config" and "codex" in op.target
    ]
    assert len(codex_notice) == 1
    assert codex_notice[0].disposition is Disposition.SKIP


def test_check_passes_for_claude_only_repo(repo: Path):
    decisions = Decisions(agents=[Agent.CLAUDE_CODE])
    apply(build_plan(repo, _facts(repo), _no_skills(), decisions=decisions), repo)
    names = {r.name: r for r in run_checks(repo)}
    assert "opencode.jsonc valid" not in names  # not required when absent
    assert names[".claude/settings.json valid"].ok
    assert names["CLAUDE.md -> AGENTS.md"].ok
    assert names["AGENTS.md section"].ok


def test_check_passes_for_codex_only_repo(repo: Path):
    decisions = Decisions(agents=[Agent.CODEX])
    apply(build_plan(repo, _facts(repo), _no_skills(), decisions=decisions), repo)
    names = {r.name: r for r in run_checks(repo)}
    assert "opencode.jsonc valid" not in names
    assert "CLAUDE.md -> AGENTS.md" not in names
    assert names["AGENTS.md section"].ok


# --- local skills: every declared slug ships a real SKILL.md -----------------
def test_local_skills_each_ship_a_skill_file():
    """Guard the LOCAL_SKILLS -> skills/**/SKILL.md seam directly.

    LOCAL_SKILLS drives `npx skills add collectiveai-team/botica --skill
    <slug>`; a slug with no matching skills/**/<slug>/SKILL.md would silently
    fail at install time in every target repo.
    """
    from pathlib import Path as _Path

    repo_root = _Path(__file__).resolve().parent.parent
    skill_files = {p.parent.name: p for p in (repo_root / "skills").glob("*/*/SKILL.md")}
    for slug in LOCAL_SKILLS:
        assert slug in skill_files, f"LOCAL_SKILLS declares {slug!r} but no matching SKILL.md"
        frontmatter = skill_files[slug].read_text(encoding="utf-8")
        assert f"name: {slug}" in frontmatter
