"""An agent runs `botica refs show` before it writes code that a rule covers (ADR 0007).

The instruction is in the texts that an agent reads: the index that the recipe `base` appends to
`AGENTS.md`, and the `AGENTS.md` of this repository.
"""

from __future__ import annotations

import re
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
INSTRUCTED = [
    ROOT / "catalog" / "recipes" / "sections" / "botica-rules.md",
    ROOT / "AGENTS.md",
]


@pytest.mark.parametrize("text_file", INSTRUCTED, ids=["template", "repo"])
def test_the_text_sends_the_agent_to_botica_refs_show_before_it_writes_code(text_file: Path):
    text = text_file.read_text(encoding="utf-8")

    starts = [match.start() for match in re.finditer("botica refs show", text)]

    assert starts, "the text does not mention `botica refs show`"
    assert any("before" in text[max(0, start - 400) : start].lower() for start in starts)
