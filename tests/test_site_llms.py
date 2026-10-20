"""The site publishes an llms.txt that lists its rules and standards with their addresses.

Asserted through the public seam of `scripts/site_llms.py`: the configuration of the site and the
exported pages go in, the text of `llms.txt` comes out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "site_llms", Path(__file__).resolve().parents[1] / "scripts" / "site_llms.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
sl = importlib.util.module_from_spec(_SPEC)
sys.modules["site_llms"] = sl
_SPEC.loader.exec_module(sl)

SITE = "https://example.org/botica/"


def site(tmp_path: Path) -> tuple[Path, Path]:
    config = tmp_path / "zensical.toml"
    config.write_text(
        '[project]\nsite_name = "Botica"\nsite_description = "The rules of botica."\n'
        f'site_url = "{SITE}"\n',
        encoding="utf-8",
    )
    docs = tmp_path / "docs"
    docs.mkdir()
    return config, docs


def llms(tmp_path: Path, config: Path, docs: Path) -> str:
    output = tmp_path / "site" / "llms.txt"
    sl.write(config, docs, output)
    return output.read_text(encoding="utf-8")


def add_rule(docs: Path, family: str, slug: str, code: str, description: str) -> None:
    page = docs / "rules" / family / f"{code.lower()}-{slug}.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    page.write_text(
        f'---\ntitle: "{slug}"\nsubtitle: "{code}"\nrule: "{slug}"\ncode: "{code}"\n'
        f'family: "{family}"\ndescription: "{description}"\n---\n\n# {slug} ({code})\n',
        encoding="utf-8",
    )


def add_standard(docs: Path, number: int, slug: str, title: str) -> None:
    page = docs / "standards" / "CES" / f"{number}-{slug}.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    page.write_text(
        f"---\nid: CES-{number}\ntitle: {title}\nsubtitle: CES-{number}\n---\n\nText.\n",
        encoding="utf-8",
    )


def test_the_file_opens_with_the_name_of_the_site_and_its_description(tmp_path: Path):
    config, docs = site(tmp_path)

    assert llms(tmp_path, config, docs).startswith("# Botica\n\n> The rules of botica.\n")


def test_each_rule_is_listed_by_code_with_the_address_of_its_page_and_its_description(
    tmp_path: Path,
):
    config, docs = site(tmp_path)
    add_rule(docs, "structure", "no-utils", "BTSTRUC002", "No catch-all modules")
    add_rule(docs, "logging", "log-no-print", "BTLOG002", "Libraries log, they don't print")

    assert (
        "\n## Rules\n\n"
        "- [log-no-print (BTLOG002)](https://example.org/botica/rules/logging/"
        "btlog002-log-no-print/): Libraries log, they don't print\n"
        "- [no-utils (BTSTRUC002)](https://example.org/botica/rules/structure/"
        "btstruc002-no-utils/): No catch-all modules\n"
    ) in llms(tmp_path, config, docs)


def test_each_standard_is_listed_by_number_with_the_address_of_its_page_and_its_title(
    tmp_path: Path,
):
    config, docs = site(tmp_path)
    add_standard(docs, 107, "skills-manifest", "Track the skills manifest")
    add_standard(docs, 16, "architecture-vocabulary", "Architectural vocabulary")
    add_standard(docs, 5, "import-linter", "Layered import direction")

    assert (
        "\n## Standards\n\n"
        "- [CES-5](https://example.org/botica/standards/CES/5-import-linter/):"
        " Layered import direction\n"
        "- [CES-16](https://example.org/botica/standards/CES/16-architecture-vocabulary/):"
        " Architectural vocabulary\n"
        "- [CES-107](https://example.org/botica/standards/CES/107-skills-manifest/):"
        " Track the skills manifest\n"
    ) in llms(tmp_path, config, docs)


def test_the_index_of_a_set_of_standards_is_not_listed(tmp_path: Path):
    config, docs = site(tmp_path)
    add_standard(docs, 71, "file-size-guard", "Keep files small")
    (docs / "standards" / "CES" / "index.md").write_text(
        "---\ntitle: CES\n---\n\n# CES\n", encoding="utf-8"
    )

    assert "standards/CES/index/" not in llms(tmp_path, config, docs)


def test_the_rules_of_two_families_are_listed_by_code_and_not_by_family(tmp_path: Path):
    config, docs = site(tmp_path)
    add_rule(docs, "architecture", "settings-module", "BTARCH002", "One settings module")
    add_rule(docs, "botica", "io-error", "BT001", "A path cannot be read")

    text = llms(tmp_path, config, docs)

    assert text.index("(BT001)") < text.index("(BTARCH002)")


def add_topic(docs: Path, topic_id: str, title: str) -> None:
    page = docs / "references" / f"{topic_id}.md"
    page.parent.mkdir(parents=True, exist_ok=True)
    page.write_text(
        f'---\ntitle: "{title}"\nsubtitle: "{topic_id}"\n---\n\n# {title}\n', encoding="utf-8"
    )


def test_each_topic_is_listed_by_id_with_the_address_of_its_page_and_its_title(tmp_path: Path):
    config, docs = site(tmp_path)
    add_topic(docs, "structured-logging", "Structured logging with structlog")
    add_topic(docs, "in-memory-adapters", "In-memory adapters for tests")

    assert (
        "\n## References\n\n"
        "- [in-memory-adapters](https://example.org/botica/references/in-memory-adapters/):"
        " In-memory adapters for tests\n"
        "- [structured-logging](https://example.org/botica/references/structured-logging/):"
        " Structured logging with structlog\n"
    ) in llms(tmp_path, config, docs)


def test_the_references_come_after_the_rules_and_the_standards(tmp_path: Path):
    config, docs = site(tmp_path)
    add_rule(docs, "structure", "no-utils", "BTSTRUC002", "No catch-all modules")
    add_standard(docs, 71, "file-size-guard", "Keep files small")
    add_topic(docs, "structured-logging", "Structured logging")

    text = llms(tmp_path, config, docs)

    assert text.index("## Rules") < text.index("## Standards") < text.index("## References")


def test_the_index_of_the_references_is_not_listed(tmp_path: Path):
    config, docs = site(tmp_path)
    add_topic(docs, "structured-logging", "Structured logging")
    (docs / "references" / "index.md").write_text("---\ntitle: References\n---\n", encoding="utf-8")

    assert "references/index/" not in llms(tmp_path, config, docs)


def test_a_site_without_topics_has_no_section_of_references(tmp_path: Path):
    config, docs = site(tmp_path)

    assert "## References" not in llms(tmp_path, config, docs)
