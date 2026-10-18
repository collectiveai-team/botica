"""An agent runs `botica refs show` before it writes code that a rule covers (ADR 0007).

The instruction is in the texts that an agent reads: the skill `engineering-rules`, the index that
the scaffolder appends to `AGENTS.md`, and the `AGENTS.md` of this repository.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
SKILL = ROOT / "skills" / "engineering" / "engineering-rules" / "SKILL.md"
INSTRUCTED = [
    SKILL,
    ROOT / "scaffolding" / "templates" / "standards-index.md",
    ROOT / "AGENTS.md",
]


@pytest.mark.parametrize("text_file", INSTRUCTED, ids=["skill", "template", "repo"])
def test_the_text_sends_the_agent_to_botica_refs_show_before_it_writes_code(text_file: Path):
    text = text_file.read_text(encoding="utf-8")

    starts = [match.start() for match in re.finditer("botica refs show", text)]

    assert starts, "the text does not mention `botica refs show`"
    assert any("before" in text[max(0, start - 400) : start].lower() for start in starts)


def test_the_skill_says_where_the_topics_are_listed_and_what_to_do_without_botica():
    text = SKILL.read_text(encoding="utf-8")
    section = text.split("## Code that a rule points at", 1)[1].split("\n## ", 1)[0]

    assert "uvx botica refs list" in section
    assert "no `botica`" in section or "without botica" in section.lower()
