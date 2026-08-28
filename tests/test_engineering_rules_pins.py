"""The engineering ruleset hand-copies this repo's config; bind the copies.

`py-ruff-format-modern` teaches the house prek setup by showing a `prek.toml`
excerpt, including the pinned `rev` for the ruff hook repo. Dependabot bumps that
pin in `prek.toml` on its own schedule and has no reason to touch a markdown rule,
so the two drift silently — the rule keeps teaching a rev the repo no longer uses.

Same posture as `test_skill_catalog.py`: a copy that cannot drift undetected is
worth more than a copy nobody re-reads. When this fails, update the rule and
recompile the skill's `AGENTS.md` from `rules/` — both carry the value.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SKILL = REPO_ROOT / "skills" / "engineering" / "engineering-rules"
RULE = SKILL / "rules" / "py-ruff-format-modern.md"
RUFF_HOOK_REPO = "https://github.com/astral-sh/ruff-pre-commit"


def _prek_rev(repo_url: str) -> str:
    data = tomllib.loads((REPO_ROOT / "prek.toml").read_text(encoding="utf-8"))
    revs = [r["rev"] for r in data.get("repos", []) if r.get("repo") == repo_url]
    assert revs, f"prek.toml declares no repo {repo_url}"
    assert len(revs) == 1, f"prek.toml declares {repo_url} {len(revs)} times"
    return revs[0]


def _doc_revs(path: Path) -> list[str]:
    # The rule shows the pin inside a fenced toml block, as `rev = "vX.Y.Z"`.
    return re.findall(r'^rev\s*=\s*"([^"]+)"', path.read_text(encoding="utf-8"), re.M)


def test_rule_quotes_the_ruff_rev_prek_actually_pins():
    doc_revs = _doc_revs(RULE)
    assert doc_revs, f"{RULE.name} no longer shows a rev pin — drop this test if that was deliberate"
    expected = _prek_rev(RUFF_HOOK_REPO)
    assert set(doc_revs) == {expected}, (
        f"{RULE.name} teaches rev {sorted(set(doc_revs))}, prek.toml pins {expected!r}. "
        "Update the rule and recompile the skill's AGENTS.md."
    )


def test_compiled_agents_md_carries_the_same_rev():
    # AGENTS.md is generated from rules/, so a rule fixed without a recompile
    # leaves the stale pin in the document agents actually skim.
    expected = _prek_rev(RUFF_HOOK_REPO)
    compiled = (SKILL / "AGENTS.md").read_text(encoding="utf-8")
    stale = [rev for rev in re.findall(r'^rev\s*=\s*"([^"]+)"', compiled, re.M) if rev != expected]
    assert not stale, (
        f"engineering-rules/AGENTS.md carries rev {sorted(set(stale))}, prek.toml pins "
        f"{expected!r} — recompile it from rules/."
    )


@pytest.mark.parametrize("path", [RULE, SKILL / "AGENTS.md"])
def test_the_pin_is_attributed_to_the_hook_repo_it_belongs_to(path: Path):
    # A bare `rev` with no nearby repo line is unmaintainable: the next reader
    # cannot tell which hook repo it pins, and this test cannot either.
    text = path.read_text(encoding="utf-8")
    assert RUFF_HOOK_REPO in text, (
        f"{path.name} pins a rev without naming {RUFF_HOOK_REPO} — the pin is unattributable"
    )
