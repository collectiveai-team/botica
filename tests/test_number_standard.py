"""A proposal carries its standard as number 0, and the merge gives it the next free number.

Asserted through the public seam of `scripts/number_standard.py`: the standards and the rules on
disk go in, the numbered files come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

_SPEC = importlib.util.spec_from_file_location(
    "number_standard", Path(__file__).resolve().parents[1] / "scripts" / "number_standard.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
ns = importlib.util.module_from_spec(_SPEC)
sys.modules["number_standard"] = ns
_SPEC.loader.exec_module(ns)


def write(root: Path, path: str, text: str = "x\n") -> None:
    target = root / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")


def read(root: Path, path: str) -> str:
    return (root / path).read_text(encoding="utf-8")


def test_the_proposed_standard_takes_the_next_free_number_of_its_set(tmp_path: Path):
    write(tmp_path, "standards/CES/120-a.md")
    write(tmp_path, "standards/CES/131-b.md")
    write(tmp_path, "standards/CES/index.md")
    write(tmp_path, "standards/ACME/400-c.md")
    write(tmp_path, "standards/CES/0-new.md", "---\nid: CES-0\ntitle: New\n---\n")
    write(tmp_path, "catalog/rules/f/r/rule.toml", 'id = "r"\nstandard = "CES-0"\n')
    write(tmp_path, "catalog/rules/f/s/rule.toml", 'id = "s"\nstandard = "CES-120"\n')

    assert ns.number(tmp_path, "CES") == "CES-132"

    assert not (tmp_path / "standards/CES/0-new.md").exists()
    assert read(tmp_path, "standards/CES/132-new.md") == "---\nid: CES-132\ntitle: New\n---\n"
    assert read(tmp_path, "catalog/rules/f/r/rule.toml") == 'id = "r"\nstandard = "CES-132"\n'
    assert read(tmp_path, "catalog/rules/f/s/rule.toml") == 'id = "s"\nstandard = "CES-120"\n'


def test_the_number_reaches_the_recipes_and_the_documentation_that_cite_the_standard(
    tmp_path: Path,
):
    write(tmp_path, "standards/CES/131-b.md")
    write(tmp_path, "standards/CES/0-new.md", "---\nid: CES-0\n---\n")
    write(tmp_path, "catalog/recipes/files/tool.yaml", "# CES-0. CES-01 stays.\n")
    write(tmp_path, "website/docs/guide/recipes.md", "the recipe (CES-0).\n")
    write(tmp_path, "docs/adr/0001-x.md", "CES-0\n")
    write(tmp_path, "README.md", "ACME-0 stays, CES-0 moves\n")
    write(tmp_path, "target/debug/build.log", "CES-0\n")
    write(tmp_path, "standards/CES/index.md", "A proposal says `id: CES-0`.\n")

    ns.number(tmp_path, "CES")

    assert read(tmp_path, "standards/CES/index.md") == "A proposal says `id: CES-0`.\n"
    assert read(tmp_path, "catalog/recipes/files/tool.yaml") == "# CES-132. CES-01 stays.\n"
    assert read(tmp_path, "website/docs/guide/recipes.md") == "the recipe (CES-132).\n"
    assert read(tmp_path, "docs/adr/0001-x.md") == "CES-132\n"
    assert read(tmp_path, "README.md") == "ACME-0 stays, CES-132 moves\n"
    assert read(tmp_path, "target/debug/build.log") == "CES-0\n"


def test_a_set_with_nothing_proposed_is_left_as_it_is(tmp_path: Path):
    write(tmp_path, "standards/CES/120-a.md")

    with pytest.raises(SystemExit, match="no standard numbered 0"):
        ns.number(tmp_path, "CES")


def test_two_proposed_standards_of_a_set_are_refused(tmp_path: Path):
    write(tmp_path, "standards/CES/0-a.md")
    write(tmp_path, "standards/CES/0-b.md")

    with pytest.raises(SystemExit, match="more than one"):
        ns.number(tmp_path, "CES")
