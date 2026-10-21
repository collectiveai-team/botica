"""The index of the standards lists every standard with its rules and its topics.

Asserted through the public seam of `scripts/site_standards.py`: the pages of the rules, the copy
of the standards and the index go in, the index that results comes out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "site_standards", Path(__file__).resolve().parents[1] / "scripts" / "site_standards.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
ss = importlib.util.module_from_spec(_SPEC)
sys.modules["site_standards"] = ss
_SPEC.loader.exec_module(ss)

HEADER = "| Standard | Title | Rules | Topics |\n|---|---|---|---|\n"


def add_index(docs: Path) -> Path:
    index = docs / "standards" / "index.md"
    index.parent.mkdir(parents=True, exist_ok=True)
    index.write_text("---\ntitle: Standards\n---\n\n# Standards\n\nIntro.\n", encoding="utf-8")
    return index


def add_standard(docs: Path, standard_set: str, number: int, title: str, topics: str = "") -> None:
    page = docs / "standards" / standard_set / f"{number}-a-standard.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    cited = f"references: {topics}\n" if topics else ""
    page.write_text(
        f"---\nid: {standard_set}-{number}\ntitle: {title}\n{cited}---\n\nText.\n",
        encoding="utf-8",
    )


def add_rule(docs: Path, family: str, slug: str, code: str, standard: str) -> None:
    page = docs / "rules" / family / f"{code.lower()}-{slug}.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    page.write_text(
        f'---\ntitle: "{slug}"\nrule: "{slug}"\ncode: "{code}"\nstandard: "{standard}"\n---\n\n'
        f"# {slug}\n\nFamily: {family} · Standard: {standard}\n",
        encoding="utf-8",
    )


def table(index: Path) -> str:
    text = index.read_text(encoding="utf-8")
    assert "\n## All standards\n\n" in text
    return text.split("\n## All standards\n\n", 1)[1]


def test_the_index_lists_each_standard_with_a_link_and_its_title(tmp_path: Path):
    index = add_index(tmp_path)
    add_standard(tmp_path, "CES", 4, "API schemas forbid extra fields")

    ss.decorate(tmp_path)

    assert table(index) == (
        HEADER + "| [CES-4](CES/4-a-standard.md) | API schemas forbid extra fields |  |  |\n"
    )
    assert index.read_text(encoding="utf-8").startswith(
        "---\ntitle: Standards\n---\n\n# Standards\n\nIntro.\n"
    )


def test_the_standards_come_by_set_and_then_by_number_and_not_as_text(tmp_path: Path):
    index = add_index(tmp_path)
    add_standard(tmp_path, "CES", 71, "Seventy-one")
    add_standard(tmp_path, "CES", 9, "Nine")
    add_standard(tmp_path, "ACME", 100, "Acme")

    ss.decorate(tmp_path)

    ids = [line.split("]")[0].removeprefix("| [") for line in table(index).splitlines()[2:]]
    assert ids == ["ACME-100", "CES-9", "CES-71"]


def test_a_standard_lists_its_rules_by_code_and_its_topics(tmp_path: Path):
    index = add_index(tmp_path)
    add_standard(tmp_path, "CES", 74, "The house logger", topics='["structured-logging", "other"]')
    add_rule(tmp_path, "logging", "log-no-print", "BTLOG002", "CES-74")
    add_rule(tmp_path, "logging", "log-get-logger", "BTLOG001", "CES-74")

    ss.decorate(tmp_path)

    row = table(index).splitlines()[2]
    rules = (
        "[`log-get-logger`](../rules/logging/btlog001-log-get-logger.md), "
        "[`log-no-print`](../rules/logging/btlog002-log-no-print.md)"
    )
    topics = (
        "[structured-logging](../references/structured-logging.md), [other](../references/other.md)"
    )
    assert row == f"| [CES-74](CES/74-a-standard.md) | The house logger | {rules} | {topics} |"


def test_a_pipe_in_a_title_does_not_split_its_cell(tmp_path: Path):
    index = add_index(tmp_path)
    add_standard(tmp_path, "CES", 4, "a | b")

    ss.decorate(tmp_path)

    assert "| a \\| b |" in table(index)


def test_a_checkout_without_an_index_of_the_standards_is_left_alone(tmp_path: Path):
    add_standard(tmp_path, "CES", 4, "A title")

    ss.decorate(tmp_path)

    assert not (tmp_path / "standards" / "index.md").exists()
