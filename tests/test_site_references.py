"""The site has a page per topic of references/, and each rule and standard lists its topics.

Asserted through the public seam of `scripts/site_references.py`: the folder `references/`, the
files it points at and the pages of the rules and the standards go in, and the pages of the site
come out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

_SPEC = importlib.util.spec_from_file_location(
    "site_references", Path(__file__).resolve().parents[1] / "scripts" / "site_references.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
sr = importlib.util.module_from_spec(_SPEC)
sys.modules["site_references"] = sr
_SPEC.loader.exec_module(sr)

SOURCE = "import os\n\ndef get_logger():\n    pass\n\nTAIL = 1\n"
SHA = "3f2a9c1d5e7b4a6c8d0e1f2a3b4c5d6e7f8a9b0c"
TOPIC = """title = "Structured logging"

[[references]]
path = "src/app.py"
lines = "3-4"
anchor = "def get_logger"
note = "the setup"
"""
README = "Read the setup.\n\n## What to read\n\nThe function.\n"


class Site:
    """A checkout with `references/` and `src/app.py`, and the `docs/` of the site."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.references = root / "references"
        self.docs = root / "docs"
        (root / "src").mkdir()
        (root / "src" / "app.py").write_text(SOURCE, encoding="utf-8")

    def topic(self, topic_id: str = "structured-logging", toml: str = TOPIC) -> None:
        folder = self.references / topic_id
        folder.mkdir(parents=True, exist_ok=True)
        (folder / "topic.toml").write_text(toml, encoding="utf-8")
        (folder / "README.md").write_text(README, encoding="utf-8")

    def rule(self, family: str, slug: str, code: str, cites: str | None = None) -> Path:
        page = self.docs / "rules" / family / f"{code.lower()}-{slug}.md"
        page.parent.mkdir(parents=True, exist_ok=True)
        cited = f"references: {cites}\n" if cites else ""
        page.write_text(
            f'---\ntitle: "{slug}"\nrule: "{slug}"\ncode: "{code}"\n{cited}---\n\n# {slug}\n',
            encoding="utf-8",
        )
        return page

    def standard(self, number: int, slug: str, cites: str | None = None) -> Path:
        page = self.docs / "standards" / "CES" / f"{number}-{slug}.md"
        page.parent.mkdir(parents=True, exist_ok=True)
        cited = f"references: {cites}\n" if cites else ""
        page.write_text(
            f"---\nid: CES-{number}\ntitle: A standard\n{cited}---\n\nText.\n", encoding="utf-8"
        )
        return page

    def build(self) -> None:
        sr.build(self.references, self.docs)

    def page(self, topic_id: str) -> str:
        return (self.docs / "references" / f"{topic_id}.md").read_text(encoding="utf-8")


@pytest.fixture
def site(tmp_path: Path) -> Site:
    return Site(tmp_path)


def test_a_topic_gets_a_page_with_its_title_and_its_readme(site: Site):
    site.topic()

    site.build()

    text = site.page("structured-logging")
    assert text.startswith(
        '---\ntitle: "Structured logging"\nsubtitle: "structured-logging"\n---\n'
    )
    assert "\n# Structured logging\n" in text
    assert "Read the setup.\n\n## What to read\n\nThe function.\n" in text


def test_a_page_reference_shows_its_address_and_its_note_without_code(site: Site):
    site.topic(
        toml='title = "T"\n\n[[references]]\nurl = "https://example.com/docs"\n'
        'note = "the documentation"\n\n[[references]]\npath = "src/app.py"\nlines = "3-4"\n'
        'note = "the setup"\n'
    )

    site.build()

    text = site.page("structured-logging")
    page = "### <https://example.com/docs>\n\nthe documentation\n\n### `src"
    assert f"\n## References\n\n{page}" in text
    assert text.count("```") == 2, "only the file has a block of code"


def test_a_reference_shows_its_path_its_lines_its_note_and_the_text_of_those_lines(site: Site):
    site.topic()

    site.build()

    text = site.page("structured-logging")
    assert (
        "\n### `src/app.py:3-4`\n\nthe setup\n\n```python\ndef get_logger():\n    pass\n```\n"
    ) in text
    assert "import os" not in text
    assert "TAIL" not in text


def test_a_reference_without_lines_shows_the_whole_file(site: Site):
    site.topic(toml='title = "T"\n\n[[references]]\npath = "src/app.py"\nnote = "all"\n')

    site.build()

    text = site.page("structured-logging")
    assert "\n### `src/app.py`\n" in text
    assert f"```python\n{SOURCE}```\n" in text


def test_the_language_of_the_block_comes_from_the_extension_of_the_file(site: Site):
    (site.root / "src" / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")
    (site.root / "src" / "notes.xyz").write_text("plain\n", encoding="utf-8")
    site.topic(
        toml='title = "T"\n\n[[references]]\npath = "src/lib.rs"\nnote = "a"\n\n'
        '[[references]]\npath = "src/notes.xyz"\nnote = "b"\n'
    )

    site.build()

    text = site.page("structured-logging")
    assert "```rust\nfn main() {}\n```" in text
    assert "```\nplain\n```" in text


@pytest.mark.parametrize(
    ("guide", "fence"),
    [
        ("# Guide\n\n```python\nx = 1\n```\n", "````"),
        ("a\n`````\nb\n``\n", "``````"),
        ("Call `x` and ``y``.\n", "```"),
    ],
)
def test_the_fence_is_longer_than_the_longest_run_of_backticks_in_the_file(
    site: Site, guide: str, fence: str
):
    (site.root / "docs").mkdir()
    (site.root / "docs" / "guide.md").write_text(guide, encoding="utf-8")
    site.topic(toml='title = "T"\n\n[[references]]\npath = "docs/guide.md"\nnote = "n"\n')

    site.build()

    text = site.page("structured-logging")
    assert f"\n{fence}markdown\n{guide}{fence}\n" in text


def test_the_fence_depends_on_the_lines_that_are_shown_and_not_on_the_rest_of_the_file(site: Site):
    (site.root / "docs").mkdir()
    (site.root / "docs" / "guide.md").write_text("plain\n```\nx\n```\n", encoding="utf-8")
    site.topic(
        toml='title = "T"\n\n[[references]]\npath = "docs/guide.md"\nlines = "1"\nnote = "n"\n'
    )

    site.build()

    text = site.page("structured-logging")
    assert "\n```markdown\nplain\n```\n" in text
    assert "````" not in text


def test_an_external_reference_shows_its_repository_its_commit_and_its_license_without_code(
    site: Site,
):
    site.topic(
        toml=f'title = "T"\n\n[[references]]\nrepo = "github.com/acme/app"\nrev = "{SHA}"\n'
        'license = "MIT"\npath = "a.py"\nlines = "1-2"\nnote = "n"\n'
    )

    site.build()

    text = site.page("structured-logging")
    assert f"\n### `a.py:1-2`\n\nn\n\ngithub.com/acme/app at {SHA} (MIT)\n" in text
    assert "```" not in text


def test_a_topic_lists_the_rules_and_the_standards_that_cite_it_with_a_link_to_each(site: Site):
    site.topic()
    site.rule("logging", "log-no-print", "BTLOG002", '["structured-logging"]')
    site.rule("logging", "log-get-logger", "BTLOG001", '["structured-logging"]')
    site.rule("structure", "no-utils", "BTSTRUC002")
    site.standard(74, "core-logger", '["structured-logging"]')
    site.standard(63, "no-utils")

    site.build()

    assert site.page("structured-logging").endswith(
        "\n## Cited by\n\n"
        "- [`log-get-logger`](../rules/logging/btlog001-log-get-logger.md) (BTLOG001)\n"
        "- [`log-no-print`](../rules/logging/btlog002-log-no-print.md) (BTLOG002)\n"
        "- [CES-74](../standards/CES/74-core-logger.md)\n"
    )


def test_the_rules_are_listed_by_code_and_the_standards_by_number(site: Site):
    site.topic()
    site.rule("logging", "zeta-rule", "BTLOG001", '["structured-logging"]')
    site.rule("logging", "alpha-rule", "BTLOG002", '["structured-logging"]')
    site.standard(74, "late", '["structured-logging"]')
    site.standard(9, "early", '["structured-logging"]')

    site.build()

    text = site.page("structured-logging")
    order = [text.index(name) for name in ("zeta-rule", "alpha-rule", "CES-9]", "CES-74]")]
    assert order == sorted(order)


def test_a_topic_is_listed_only_where_it_is_cited(site: Site):
    site.topic("structured-logging")
    site.topic("in-memory-adapters")
    site.rule("logging", "log-no-print", "BTLOG002", '["structured-logging"]')

    site.build()

    assert "log-no-print" in site.page("structured-logging")
    assert "Cited by" not in site.page("in-memory-adapters")


def test_a_topic_that_nothing_cites_has_no_list_of_citations(site: Site):
    site.topic()
    site.rule("structure", "no-utils", "BTSTRUC002")

    site.build()

    assert "Cited by" not in site.page("structured-logging")


def test_a_rule_lists_the_topics_it_cites_with_a_link_to_each_page(site: Site):
    site.topic("structured-logging")
    site.topic("in-memory-adapters", TOPIC.replace("Structured logging", "In-memory adapters"))
    rule = site.rule(
        "logging", "log-no-print", "BTLOG002", '["structured-logging", "in-memory-adapters"]'
    )

    site.build()

    assert rule.read_text(encoding="utf-8").endswith(
        "\n## References\n\n"
        "- [Structured logging](../../references/structured-logging.md)\n"
        "- [In-memory adapters](../../references/in-memory-adapters.md)\n"
    )


def test_a_standard_lists_the_topics_it_cites_with_a_link_to_each_page(site: Site):
    site.topic()
    page = site.standard(74, "core-logger", '["structured-logging"]')

    site.build()

    assert page.read_text(encoding="utf-8").endswith(
        "\nText.\n\n## References\n\n"
        "- [Structured logging](../../references/structured-logging.md)\n"
    )


def test_a_rule_and_a_standard_that_cite_nothing_are_left_as_they_are(site: Site):
    site.topic()
    rule = site.rule("structure", "no-utils", "BTSTRUC002")
    standard = site.standard(63, "no-utils")
    before = (rule.read_text(encoding="utf-8"), standard.read_text(encoding="utf-8"))

    site.build()

    assert (rule.read_text(encoding="utf-8"), standard.read_text(encoding="utf-8")) == before


def test_the_index_lists_the_topics_by_id_with_a_link_and_the_title(site: Site):
    site.topic("structured-logging")
    site.topic("in-memory-adapters", TOPIC.replace("Structured logging", "In-memory adapters"))

    site.build()

    index = (site.docs / "references" / "index.md").read_text(encoding="utf-8")
    assert index.startswith("---\ntitle: References\n---\n\n# References\n")
    assert (
        "\n- [in-memory-adapters](in-memory-adapters.md): In-memory adapters\n"
        "- [structured-logging](structured-logging.md): Structured logging\n"
    ) in index


def test_a_citation_of_a_topic_that_does_not_exist_stops_the_build(site: Site):
    site.topic()
    site.rule("logging", "log-no-print", "BTLOG002", '["ghost"]')

    with pytest.raises(ValueError, match=r"log-no-print.*ghost"):
        site.build()


def test_the_index_of_a_set_of_standards_is_not_read_as_a_standard(site: Site):
    site.topic()
    index = site.docs / "standards" / "CES" / "index.md"
    index.parent.mkdir(parents=True)
    index.write_text("---\ntitle: CES\n---\n\n# CES\n", encoding="utf-8")

    site.build()

    assert index.read_text(encoding="utf-8") == "---\ntitle: CES\n---\n\n# CES\n"


def test_a_checkout_without_references_gets_no_pages(site: Site):
    site.build()

    assert not (site.docs / "references").exists()
