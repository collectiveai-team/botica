"""Catalog tests: components.py is the single source of truth for the skill set.

The upstream skill list was hand-copied into README.md (twice) and guide.md, and
drifted the moment upstream renamed a skill — the installer skips unknown names
without failing, so the breakage was silent. These tests bind every copy to
`scaffolding.components`.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

import pytest

from scaffolding.engine import build_plan
from scaffolding.facts import detect
from scaffolding.settings import Settings
from scaffolding.skills import (
    LOCAL_SKILLS,
    MATTPOCOCK_REF,
    MATTPOCOCK_REPO,
    MATTPOCOCK_SKILLS,
    MATTPOCOCK_SOURCE,
)

REPO_ROOT = Path(__file__).resolve().parent.parent
DOCS_WITH_INSTALL_COMMAND = ["README.md", "guide.md"]
# The catalog prose block in README.md, between its heading and the next one.
CATALOG_HEADING = "## Upstream skills from Matt Pocock"


def _doc(name: str) -> str:
    return (REPO_ROOT / name).read_text(encoding="utf-8")


def _local_skill_root(name: str) -> Path | None:
    # Local skills are filed under a category directory (skills/<category>/<name>),
    # and the category is not part of the installed name — the `skills` CLI flattens
    # them into .agents/skills. Resolve by name, not by a hardcoded category.
    return next((p.parent for p in REPO_ROOT.glob(f"skills/*/{name}/SKILL.md")), None)


def test_source_pins_a_ref_as_a_fragment():
    # `owner/repo@tag` is parsed by the skills CLI as a skill-name filter, not a
    # ref, and installs from the default branch without warning.
    assert f"{MATTPOCOCK_REPO}#{MATTPOCOCK_REF}" == MATTPOCOCK_SOURCE
    assert "@" not in MATTPOCOCK_SOURCE


def test_planned_command_installs_the_pinned_source_and_catalog(tmp_path: Path):
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    facts = detect(tmp_path, probe_visibility=False)
    if not facts.has_npx:
        pytest.skip("npx not available; the skills component is skipped without it")
    plan = build_plan(tmp_path, facts, Settings(skip_varlock=True))
    cmds = [op.cmd or [] for op in plan.ops if op.component == "skills"]
    assert not [c for c in cmds if MATTPOCOCK_REPO in c], (
        "the upstream install must go through the pinned source, not the bare repo"
    )
    cmd = next(c for c in cmds if MATTPOCOCK_SOURCE in c)
    # The seed groups one `skills add` per source and sorts the names within it,
    # so compare the set the command installs, not the order it lists them in.
    assert cmd[cmd.index("--skill") + 1 :] == sorted(MATTPOCOCK_SKILLS)


@pytest.mark.parametrize("name", DOCS_WITH_INSTALL_COMMAND)
def test_docs_quote_the_pinned_source(name: str):
    assert f"'{MATTPOCOCK_SOURCE}'" in _doc(name), (
        f"{name} must quote the pinned source; bump it in scaffolding/components.py"
    )
    assert f"add {MATTPOCOCK_REPO} " not in _doc(name), f"{name} has an unpinned install command"


@pytest.mark.parametrize("name", DOCS_WITH_INSTALL_COMMAND)
def test_docs_carry_the_exact_skill_list(name: str):
    assert " ".join(MATTPOCOCK_SKILLS) in _doc(name), (
        f"{name} install command is out of sync with MATTPOCOCK_SKILLS"
    )


@pytest.mark.parametrize("name", DOCS_WITH_INSTALL_COMMAND)
def test_docs_carry_the_exact_local_skill_list(name: str):
    assert " ".join(LOCAL_SKILLS) in _doc(name), (
        f"{name} install command is out of sync with LOCAL_SKILLS"
    )


def test_readme_catalog_lists_every_upstream_skill():
    readme = _doc("README.md")
    start = readme.index(CATALOG_HEADING)
    end = readme.index("\n## ", start + 1)
    listed = set(re.findall(r"`([a-z0-9-]+)`", readme[start:end]))
    assert listed == set(MATTPOCOCK_SKILLS), (
        "README catalog and MATTPOCOCK_SKILLS disagree: "
        f"only in catalog={sorted(listed - set(MATTPOCOCK_SKILLS))}, "
        f"only in code={sorted(set(MATTPOCOCK_SKILLS) - listed)}"
    )


def test_local_skills_exist_on_disk():
    for skill in LOCAL_SKILLS:
        assert _local_skill_root(skill) is not None, (
            f"LOCAL_SKILLS names {skill}, which has no SKILL.md under skills/*/"
        )


def test_user_invoked_local_skills_carry_codex_policy():
    # Codex reads agents/openai.yaml, not the Claude-style frontmatter key, so a
    # user-invoked skill without the policy block is implicitly invokable there.
    for skill in LOCAL_SKILLS:
        root = _local_skill_root(skill)
        assert root is not None, f"LOCAL_SKILLS names {skill}, which has no SKILL.md"
        frontmatter = (root / "SKILL.md").read_text(encoding="utf-8")
        codex = root / "agents" / "openai.yaml"
        assert codex.is_file(), f"{skill} is missing agents/openai.yaml"
        user_invoked = "disable-model-invocation: true" in frontmatter
        has_policy = "allow_implicit_invocation: false" in codex.read_text(encoding="utf-8")
        assert has_policy == user_invoked, (
            f"{skill}: SKILL.md and agents/openai.yaml disagree on model invocation"
        )


def test_ask_user_routes_every_installed_skill():
    routed = _doc("skills/productivity/ask-user/SKILL.md")
    # ask-user is the router; it should not need to route itself.
    expected = (set(MATTPOCOCK_SKILLS) | set(LOCAL_SKILLS)) - {"ask-user"}
    missing = sorted(name for name in expected if f"/{name}" not in routed)
    assert not missing, f"ask-user does not route installed skills: {missing}"
