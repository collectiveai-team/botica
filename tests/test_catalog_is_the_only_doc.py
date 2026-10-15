"""The catalog of botica is the only documentation of the rules that botica measures.

A rule that `botica lint` checks deterministically is described once, in the README of its
folder under `lint/rules`. Nothing else may describe it again: not the index of standards, not a
detail file, not the skill. The CES a rule cites (`standard` in its `rule.toml`) is how the copies
are found, since a detail file or an index entry is named after a CES, not always after a slug.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest

from scaffolding.components import STANDARDS_RULE_DETAILS
from scaffolding.templates_registry import template_text

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / "lint" / "rules"
SKILL_RULES = ROOT / "skills" / "engineering" / "engineering-rules" / "references"

# Rules of the skill that say what a catalog rule measures, under a name that is not its slug.
# They were removed once; this keeps them from coming back.
SKILL_RULES_THAT_REPEATED_THE_CATALOG = (
    "general-repo-shape",
    "spaghetti-large-file-thresholds",
    "spaghetti-no-utils-dumping",
    "pylayout-cli-typer-rich",
    "log-get-logger-structlog",
)

CES_ENTRY = re.compile(r"^- \*\*CES-(\d+) · ", re.M)
CES_HEADING = re.compile(r"\A# CES-(\d+) · ")
CES_ROW = re.compile(r"^\| \*\*CES-(\d+)\*\* \|", re.M)


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


def _detail_dirs() -> list[Path]:
    return [ROOT / "scaffolding" / "templates" / "agents-rules", ROOT / ".agents" / "rules"]


def test_the_catalog_is_found():
    assert {"file-size-guard", "log-no-print", "no-dict-alias"} <= SLUGS
    assert {4, 18, 32, 45, 46, 63, 67, 71, 76, 79} <= MEASURED_CES


@pytest.mark.parametrize("directory", _detail_dirs(), ids=lambda path: path.parent.name)
def test_no_detail_file_describes_a_rule_the_catalog_measures(directory: Path):
    found = []
    for path in sorted(directory.glob("*.md")):
        heading = CES_HEADING.match(path.read_text(encoding="utf-8"))
        cites_a_measured_ces = bool(heading) and int(heading.group(1)) in MEASURED_CES
        if path.stem in SLUGS or cites_a_measured_ces:
            found.append(path.name)

    assert not found, f"described in {directory.name} and in the catalog: {found}"


def test_the_standards_component_ships_no_detail_of_a_measured_rule():
    shipped = [
        name
        for name in STANDARDS_RULE_DETAILS
        if name in SLUGS or _heading_ces(template_text(f"agents-rules/{name}.md")) in MEASURED_CES
    ]

    assert not shipped, f"STANDARDS_RULE_DETAILS names rules of the catalog: {shipped}"


def _heading_ces(text: str) -> int | None:
    heading = CES_HEADING.match(text)
    return int(heading.group(1)) if heading else None


@pytest.mark.parametrize(
    "index",
    [ROOT / "scaffolding" / "templates" / "standards-index.md", ROOT / "AGENTS.md"],
    ids=["template", "repo"],
)
def test_no_index_lists_a_standard_the_catalog_measures(index: Path):
    listed = {int(n) for n in CES_ENTRY.findall(index.read_text(encoding="utf-8"))}

    assert not listed & MEASURED_CES, f"{index.name} lists {sorted(listed & MEASURED_CES)}"


@pytest.mark.parametrize(
    "index",
    [ROOT / "scaffolding" / "templates" / "standards-index.md", ROOT / "AGENTS.md"],
    ids=["template", "repo"],
)
def test_the_index_sends_the_reader_to_the_catalog_without_naming_a_rule(index: Path):
    text = index.read_text(encoding="utf-8")

    assert "uvx botica rules" in text
    assert "uvx botica explain" in text


def test_the_table_of_shipped_standards_leaves_the_measured_ones_to_the_catalog():
    text = (ROOT / "docs" / "engineering-standards.md").read_text(encoding="utf-8")
    rows = {int(n) for n in CES_ROW.findall(text)}

    assert not rows & MEASURED_CES, f"rows for {sorted(rows & MEASURED_CES)}"


def test_the_skill_does_not_repeat_a_rule_of_the_catalog():
    names = {path.stem for path in SKILL_RULES.glob("*.md")}
    sections = (SKILL_RULES / "_sections.md").read_text(encoding="utf-8")

    repeated = sorted(
        (names & SLUGS)
        | {name for name in SKILL_RULES_THAT_REPEATED_THE_CATALOG if name in names}
        | {name for name in SKILL_RULES_THAT_REPEATED_THE_CATALOG if f"`{name}`" in sections}
    )

    assert not repeated, f"the skill repeats the catalog: {repeated}"


AST_GREP = re.compile(r"ast-grep|sgconfig", re.I)


def _normative_texts() -> list[Path]:
    """List what is installed in a target repo or read as the house standard, and the READMEs."""
    templates = ROOT / "scaffolding" / "templates"
    github = ROOT / ".github"
    paths = [
        *templates.rglob("*.md"),
        *templates.rglob("*.toml"),
        *templates.rglob("*.yml"),
        *github.rglob("*.md"),
        *github.rglob("*.yml"),
        *(ROOT / ".agents" / "rules").glob("*.md"),
        *(ROOT / "skills").rglob("*.md"),
        *CATALOG.glob("*/*/README.md"),
        ROOT / "AGENTS.md",
        ROOT / "docs" / "engineering-standards.md",
    ]
    return sorted(set(paths))


def test_no_normative_text_describes_the_stack_or_a_rule_with_ast_grep():
    """Botica is the tool; ast-grep is how some of its rules are written, and nobody has to know.

    `compat-ast-grep-ignore` is documented once, in `lint/README.md`, not in every rule README.
    """
    found = {
        str(path.relative_to(ROOT)): sorted({m.lower() for m in AST_GREP.findall(text)})
        for path in _normative_texts()
        if AST_GREP.search(text := path.read_text(encoding="utf-8"))
    }

    assert not found, f"texts that name ast-grep: {found}"


def test_compat_ast_grep_ignore_is_documented_once_in_the_readme_of_botica():
    text = (ROOT / "lint" / "README.md").read_text(encoding="utf-8")

    assert "# ast-grep-ignore:" in text
    assert "compat-ast-grep-ignore = true" in text


def test_a_readme_holds_the_prose_of_a_rule_and_not_what_its_rule_toml_says():
    """The title, the status, the family and the standard of a rule come from `rule.toml`."""
    repeated = sorted(
        str(readme.relative_to(CATALOG))
        for readme in CATALOG.glob("*/*/README.md")
        if not readme.read_text(encoding="utf-8").startswith("## ")
    )

    assert not repeated, f"READMEs that open with something other than a section: {repeated}"
