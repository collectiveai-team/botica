"""Catalog tests: the `[[skills]]` steps of the recipe `base` are the skill set.

The skills of this repository (`collectiveai-team/botica`) exist under `skills/`, each one says
to Codex what its frontmatter says to Claude, and `ask-user` routes every installed skill.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
BASE = REPO_ROOT / "lint" / "recipes" / "base.toml"
LOCAL_SOURCE = "collectiveai-team/botica"
UPSTREAM_REPO = "mattpocock/skills"


def _skills_steps() -> list[dict]:
    return tomllib.loads(BASE.read_text(encoding="utf-8"))["skills"]


def _names(source_prefix: str) -> list[str]:
    return [
        name
        for step in _skills_steps()
        if step["source"].startswith(source_prefix)
        for name in step["names"]
    ]


def _local_skill_root(name: str) -> Path | None:
    # Local skills are filed under a category directory (skills/<category>/<name>),
    # and the category is not part of the installed name — the `skills` CLI flattens
    # them into .agents/skills. Resolve by name, not by a hardcoded category.
    return next((p.parent for p in REPO_ROOT.glob(f"skills/*/{name}/SKILL.md")), None)


def test_the_upstream_source_pins_a_ref_as_a_fragment():
    # `owner/repo@tag` is parsed by the skills CLI as a skill-name filter, not a
    # ref, and installs from the default branch without warning.
    sources = [s["source"] for s in _skills_steps() if s["source"].startswith(UPSTREAM_REPO)]
    assert len(sources) == 1, sources
    assert sources[0].startswith(f"{UPSTREAM_REPO}#v"), sources[0]
    assert "@" not in sources[0]


def test_local_skills_exist_on_disk():
    local = _names(LOCAL_SOURCE)
    assert local
    for skill in local:
        assert _local_skill_root(skill) is not None, (
            f"base.toml names {skill}, which has no SKILL.md under skills/*/"
        )


def test_user_invoked_local_skills_carry_codex_policy():
    # Codex reads agents/openai.yaml, not the Claude-style frontmatter key, so a
    # user-invoked skill without the policy block is implicitly invokable there.
    for skill in _names(LOCAL_SOURCE):
        root = _local_skill_root(skill)
        assert root is not None, f"base.toml names {skill}, which has no SKILL.md"
        frontmatter = (root / "SKILL.md").read_text(encoding="utf-8")
        codex = root / "agents" / "openai.yaml"
        assert codex.is_file(), f"{skill} is missing agents/openai.yaml"
        user_invoked = "disable-model-invocation: true" in frontmatter
        has_policy = "allow_implicit_invocation: false" in codex.read_text(encoding="utf-8")
        assert has_policy == user_invoked, (
            f"{skill}: SKILL.md and agents/openai.yaml disagree on model invocation"
        )


def test_ask_user_routes_every_installed_skill():
    routed = (REPO_ROOT / "skills/productivity/ask-user/SKILL.md").read_text(encoding="utf-8")
    # ask-user is the router; it should not need to route itself.
    expected = (set(_names(UPSTREAM_REPO)) | set(_names(LOCAL_SOURCE))) - {"ask-user"}
    missing = sorted(name for name in expected if f"/{name}" not in routed)
    assert not missing, f"ask-user does not route installed skills: {missing}"


def test_pr_is_installed_only_from_the_internal_catalog():
    # The house `pr` replaces the embedded body template of upstream `pr` with the target
    # repository's `.github/pull_request_template.md`, so upstream must not install it too.
    assert "pr" in _names(LOCAL_SOURCE)
    assert "pr" not in _names(UPSTREAM_REPO)
