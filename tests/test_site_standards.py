"""The page of a standard on the site lists the rules that implement it and is subtitled by its id.

Asserted through the public seam of `scripts/site_standards.py`: the exported pages of the rules and
the copy of the standards go in, the copy of the standards that results comes out (CES-65).
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


def add_rule(docs: Path, family: str, slug: str, code: str, standard: str | None) -> None:
    page = docs / "rules" / family / f"{code.lower()}-{slug}.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    cited = f'standard: "{standard}"\n' if standard else ""
    identity = f"Family: {family} · Standard: {standard} · Languages: all" if standard else "-"
    page.write_text(
        f'---\ntitle: "{slug}"\nsubtitle: "{code}"\nrule: "{slug}"\ncode: "{code}"\n{cited}---\n\n'
        f"# {slug} ({code})\n\n{identity}\n",
        encoding="utf-8",
    )


def rule_text(docs: Path, family: str, slug: str, code: str) -> str:
    return (docs / "rules" / family / f"{code.lower()}-{slug}.md").read_text(encoding="utf-8")


def add_standard(docs: Path, number: int, slug: str = "a-standard") -> Path:
    page = docs / "standards" / "CES" / f"{number}-{slug}.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    page.write_text(
        f"---\nid: CES-{number}\ntitle: A standard\n---\n\nWhat it requires.\n", encoding="utf-8"
    )
    return page


def test_a_standard_lists_the_rules_that_name_it_by_code_with_a_link_to_each(tmp_path: Path):
    page = add_standard(tmp_path, 79)
    add_rule(tmp_path, "data-models", "no-dict-call-return", "BTDM003", "CES-79")
    add_rule(tmp_path, "data-models", "no-dict-alias", "BTDM002", "CES-79")

    ss.decorate(tmp_path)

    text = page.read_text(encoding="utf-8")
    assert text.endswith(
        "\n## Rules\n\n"
        "- [`no-dict-alias`](../../rules/data-models/btdm002-no-dict-alias.md) (BTDM002)\n"
        "- [`no-dict-call-return`](../../rules/data-models/btdm003-no-dict-call-return.md)"
        " (BTDM003)\n"
    )


def test_the_rules_of_two_families_are_listed_by_code_and_not_by_family(tmp_path: Path):
    page = add_standard(tmp_path, 32)
    add_rule(tmp_path, "architecture", "late", "BTZ001", "CES-32")
    add_rule(tmp_path, "structure", "early", "BTA001", "CES-32")

    ss.decorate(tmp_path)

    text = page.read_text(encoding="utf-8")
    assert text.index("`early`") < text.index("`late`")


def test_a_standard_does_not_list_the_rules_of_another_standard_or_of_none(tmp_path: Path):
    page = add_standard(tmp_path, 71)
    add_rule(tmp_path, "structure", "file-size-guard", "BTSTRUC001", "CES-71")
    add_rule(tmp_path, "structure", "no-utils", "BTSTRUC002", "CES-63")
    add_rule(tmp_path, "structure", "no-tests-in-package", "BTSTRUC005", None)

    ss.decorate(tmp_path)

    text = page.read_text(encoding="utf-8")
    assert "file-size-guard" in text
    assert "no-utils" not in text
    assert "no-tests-in-package" not in text


def test_a_standard_that_no_rule_names_gets_no_list(tmp_path: Path):
    page = add_standard(tmp_path, 77)
    add_rule(tmp_path, "structure", "file-size-guard", "BTSTRUC001", "CES-71")

    ss.decorate(tmp_path)

    assert "## Rules" not in page.read_text(encoding="utf-8")


def test_a_standard_is_subtitled_by_its_id_in_the_front_matter(tmp_path: Path):
    page = add_standard(tmp_path, 71)

    ss.decorate(tmp_path)

    assert page.read_text(encoding="utf-8").startswith(
        "---\nid: CES-71\ntitle: A standard\nsubtitle: CES-71\n---\n"
    )


def test_the_index_of_a_set_is_left_as_it_is(tmp_path: Path):
    index = tmp_path / "standards" / "CES" / "index.md"
    index.parent.mkdir(parents=True)
    index.write_text("---\ntitle: CES\n---\n\n# CES\n", encoding="utf-8")
    add_rule(tmp_path, "structure", "file-size-guard", "BTSTRUC001", "CES-71")

    ss.decorate(tmp_path)

    assert index.read_text(encoding="utf-8") == "---\ntitle: CES\n---\n\n# CES\n"


def test_a_standard_of_another_set_is_matched_by_its_own_prefix(tmp_path: Path):
    acme = tmp_path / "standards" / "ACME" / "3-own.md"
    acme.parent.mkdir(parents=True)
    acme.write_text("---\nid: ACME-3\ntitle: Own\n---\n\nText.\n", encoding="utf-8")
    add_rule(tmp_path, "structure", "file-size-guard", "BTSTRUC001", "CES-3")
    add_rule(tmp_path, "structure", "no-utils", "BTSTRUC002", "ACME-3")

    ss.decorate(tmp_path)

    text = acme.read_text(encoding="utf-8")
    assert "no-utils" in text
    assert "file-size-guard" not in text
    assert "(../../rules/structure/btstruc002-no-utils.md)" in text


def test_a_rule_links_the_standard_it_names_to_the_page_of_that_standard(tmp_path: Path):
    add_standard(tmp_path, 71, "file-size-guard")
    add_rule(tmp_path, "structure", "file-size-guard", "BTSTRUC001", "CES-71")

    ss.decorate(tmp_path)

    assert (
        "Family: structure · Standard: [CES-71](../../standards/CES/71-file-size-guard.md)"
        " · Languages: all"
    ) in rule_text(tmp_path, "structure", "file-size-guard", "BTSTRUC001")


def test_a_rule_that_names_a_standard_without_a_page_keeps_the_plain_text(tmp_path: Path):
    add_standard(tmp_path, 71, "file-size-guard")
    add_rule(tmp_path, "structure", "no-utils", "BTSTRUC002", "CES-63")

    ss.decorate(tmp_path)

    assert "Standard: CES-63 ·" in rule_text(tmp_path, "structure", "no-utils", "BTSTRUC002")


def test_a_rule_links_a_standard_of_another_set_to_its_own_folder(tmp_path: Path):
    acme = tmp_path / "standards" / "ACME" / "3-own.md"
    acme.parent.mkdir(parents=True)
    acme.write_text("---\nid: ACME-3\ntitle: Own\n---\n\nText.\n", encoding="utf-8")
    add_rule(tmp_path, "structure", "no-utils", "BTSTRUC002", "ACME-3")

    ss.decorate(tmp_path)

    assert "[ACME-3](../../standards/ACME/3-own.md)" in rule_text(
        tmp_path, "structure", "no-utils", "BTSTRUC002"
    )
