"""The catalog of botica is the only documentation of the rules that botica measures.

A rule that `botica lint` checks deterministically is described once, in the README of its
folder under `catalog/rules`. Nothing else may describe it again: not the index of standards, not a
detail file. The CES a rule cites (`standard` in its `rule.toml`) is how the copies
are found, since a detail file or an index entry is named after a CES, not always after a slug.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / "catalog" / "rules"

CES_ENTRY = re.compile(r"^- \*\*CES-(\d+) · ", re.M)


def _catalog() -> list[tuple[str, int | None]]:
    """Every rule of the catalog: its slug and the number of the CES it cites, if any."""
    rules = []
    for rule_toml in sorted(CATALOG.glob("*/*/rule.toml")):
        data = tomllib.loads(rule_toml.read_text(encoding="utf-8"))
        standard = data.get("standard", "")
        number = standard.removeprefix("CES-") if standard.startswith("CES-") else None
        rules.append((data["id"], int(number) if number else None))
    return rules


SLUGS = {slug for slug, _ in _catalog()}
MEASURED_CES = {ces for _, ces in _catalog() if ces is not None}


def _standard_files() -> set[str]:
    """Return the `<set>-<number>` of every standard written under `standards/`."""
    return {
        f"{path.parent.name}-{path.name.split('-', 1)[0]}"
        for path in (ROOT / "standards").glob("*/[0-9]*.md")
    }


def test_every_standard_a_rule_cites_is_a_file():
    cited = {
        tomllib.loads(rule_toml.read_text(encoding="utf-8"))["standard"]
        for rule_toml in CATALOG.glob("*/*/rule.toml")
        if "standard" in tomllib.loads(rule_toml.read_text(encoding="utf-8"))
    }

    assert cited, "no rule cites a standard: drop this test if that was deliberate"
    assert not cited - _standard_files(), f"no file for: {sorted(cited - _standard_files())}"


def test_the_catalog_is_found():
    assert {"file-size-guard", "log-no-print", "no-dict-alias"} <= SLUGS
    assert {4, 18, 32, 45, 46, 63, 67, 71, 76, 79} <= MEASURED_CES


@pytest.mark.parametrize(
    "index",
    [ROOT / "catalog" / "recipes" / "sections" / "botica-rules.md", ROOT / "AGENTS.md"],
    ids=["template", "repo"],
)
def test_agents_md_lists_no_standard(index: Path):
    """`AGENTS.md` holds the rules that the export writes into its block, never a standard."""
    listed = CES_ENTRY.findall(index.read_text(encoding="utf-8"))

    assert not listed, f"{index.name} lists CES-{', CES-'.join(listed)}"


@pytest.mark.parametrize(
    "index",
    [ROOT / "catalog" / "recipes" / "sections" / "botica-rules.md", ROOT / "AGENTS.md"],
    ids=["template", "repo"],
)
def test_agents_md_has_the_block_of_the_rules_and_sends_the_agent_to_botica_explain(index: Path):
    text = index.read_text(encoding="utf-8")

    assert "<!-- botica:rules -->" in text
    assert "<!-- /botica:rules -->" in text
    assert "uvx botica explain" in text


AST_GREP = re.compile(r"ast-grep|sgconfig", re.I)


def _normative_texts() -> list[Path]:
    """List what is installed in a target repo or read as the house standard, and the READMEs."""
    templates = ROOT / "catalog" / "recipes"
    github = ROOT / ".github"
    paths = [
        *templates.rglob("*.md"),
        *templates.rglob("*.toml"),
        *templates.rglob("*.yml"),
        *github.rglob("*.md"),
        *github.rglob("*.yml"),
        *(ROOT / "skills").rglob("*.md"),
        *CATALOG.glob("*/*/README.md"),
        ROOT / "AGENTS.md",
    ]
    return sorted(set(paths))


def test_no_normative_text_describes_the_stack_or_a_rule_with_ast_grep():
    """Botica is the tool; ast-grep is how some of its rules are written, and nobody has to know.

    That an `ast-grep-ignore` comment suppresses nothing is said once, in ADR 0003.
    """
    found = {
        str(path.relative_to(ROOT)): sorted({m.lower() for m in AST_GREP.findall(text)})
        for path in _normative_texts()
        if AST_GREP.search(text := path.read_text(encoding="utf-8"))
    }

    assert not found, f"texts that name ast-grep: {found}"


CES_CODE = re.compile(r"\bCES-\d+")


def test_the_text_of_a_rule_names_no_standard():
    """A rule says what it asks; its standard is the `standard` field of its `rule.toml`."""
    texts = [*CATALOG.glob("*/*/README.md"), *CATALOG.glob("*/*/*.yml")]
    questions = [
        path
        for path in CATALOG.glob("*/*/rule.toml")
        if any(
            CES_CODE.search(str(implementation.get("question", "")))
            for implementation in tomllib.loads(path.read_text(encoding="utf-8")).get(
                "implementations", []
            )
        )
    ]
    named = sorted(
        str(path.relative_to(CATALOG))
        for path in texts
        if CES_CODE.search(path.read_text(encoding="utf-8"))
    ) + sorted(str(path.relative_to(CATALOG)) for path in questions)

    assert not named, f"rule texts that name a standard: {named}"


def test_a_readme_holds_the_prose_of_a_rule_and_not_what_its_rule_toml_says():
    """The title, the status, the family and the standard of a rule come from `rule.toml`."""
    repeated = sorted(
        str(readme.relative_to(CATALOG))
        for readme in CATALOG.glob("*/*/README.md")
        if not readme.read_text(encoding="utf-8").startswith("## ")
    )

    assert not repeated, f"READMEs that open with something other than a section: {repeated}"
