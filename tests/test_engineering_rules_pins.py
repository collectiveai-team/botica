"""The engineering ruleset hand-copies this repo's config; bind the copies.

`py-ruff-format-modern` teaches the house prek setup by showing a `prek.toml`
excerpt, including the pinned `rev` of the ruff hook repo and of the botica mirror. Dependabot
bumps the ruff pin in `prek.toml` on its own schedule, and a release moves the botica one by hand;
neither has a reason to touch a markdown rule, so the copies drift silently — the rule keeps
teaching a rev the repo no longer uses.

Same posture as `test_skill_catalog.py`: a copy that cannot drift undetected is
worth more than a copy nobody re-reads. When this fails, update the rule.

A rule with a CES also ships in `scaffolding/templates/agents-rules/`, and the skill carries it
for the repositories that have no `.agents/rules/`. The two copies must say the same.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
SKILL = REPO_ROOT / "skills" / "engineering" / "engineering-rules"
RULE = SKILL / "references" / "py-ruff-format-modern.md"
RUFF_HOOK_REPO = "https://github.com/astral-sh/ruff-pre-commit"
BOTICA_HOOK_REPO = "https://github.com/collectiveai-team/botica-pre-commit"
HOOK_REPOS = [RUFF_HOOK_REPO, BOTICA_HOOK_REPO]


def _prek_rev(repo_url: str) -> str:
    data = tomllib.loads((REPO_ROOT / "prek.toml").read_text(encoding="utf-8"))
    revs = [r["rev"] for r in data.get("repos", []) if r.get("repo") == repo_url]
    assert revs, f"prek.toml declares no repo {repo_url}"
    assert len(revs) == 1, f"prek.toml declares {repo_url} {len(revs)} times"
    return revs[0]


def _doc_revs(path: Path, repo_url: str) -> list[str]:
    # The rule shows each pin inside a fenced toml block, as `repo = "<url>"` then `rev = "vX.Y.Z"`.
    pattern = rf'^repo = "{re.escape(repo_url)}"\nrev = "([^"]+)"'
    return re.findall(pattern, path.read_text(encoding="utf-8"), re.M)


@pytest.mark.parametrize("repo_url", HOOK_REPOS)
def test_rule_quotes_the_rev_prek_actually_pins(repo_url: str):
    doc_revs = _doc_revs(RULE, repo_url)
    assert doc_revs, (
        f"{RULE.name} no longer shows a rev pin for {repo_url} — drop this test if that was "
        "deliberate"
    )
    expected = _prek_rev(repo_url)
    assert set(doc_revs) == {expected}, (
        f"{RULE.name} teaches rev {sorted(set(doc_revs))} for {repo_url}, prek.toml pins "
        f"{expected!r}. Update the rule."
    )


TEMPLATE_RULES = REPO_ROOT / "scaffolding" / "templates" / "agents-rules"


def _body(path: Path) -> str:
    """Return a rule without the frontmatter the skill adds to it."""
    text = path.read_text(encoding="utf-8")
    if text.startswith("---\n"):
        text = text.split("\n---\n", 1)[1]
    return text.strip()


def test_a_rule_the_skill_shares_with_the_templates_says_what_the_template_says():
    shared = sorted(
        rule.name
        for rule in TEMPLATE_RULES.glob("*.md")
        if (SKILL / "references" / rule.name).exists()
    )
    assert shared, "no rule is in both places: drop this test if that was deliberate"

    drifted = [
        name
        for name in shared
        if _body(SKILL / "references" / name) != _body(TEMPLATE_RULES / name)
    ]

    assert not drifted, f"the skill says something else than the template: {drifted}"
