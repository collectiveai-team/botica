"""The site has a page per topic of references/, and each rule and standard lists its topics.

Asserted through the public seam of `scripts/site_references.py`: the folder `references/`, the
files it points at and the pages of the rules and the standards go in, and the pages of the site
come out (CES-65).
"""

from __future__ import annotations

import pytest
from site_references_support import FILE, SHA, SOURCE, Ref, Site


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
        references=[Ref(url="https://example.com/docs", note="the documentation"), FILE],
    )

    site.build()

    text = site.page("structured-logging")
    page = "### <https://example.com/docs>\n\nthe documentation\n\n### `src"
    assert f"\n## References\n\n{page}" in text
    assert text.count("```") == 2, "only the file has a block of code"


def test_a_page_reference_shows_its_quote_as_a_block_quote_after_its_note(site: Site):
    site.topic(
        references=[
            Ref(url="https://example.com/docs#setup", note="setup", quote="\n First.\n\nSecond.\n"),
            Ref(url="https://example.com/docs#next", note="next"),
        ]
    )

    site.build()

    text = site.page("structured-logging")
    assert (
        "### <https://example.com/docs#setup>\n\nsetup\n\n> First.\n>\n> Second.\n\n### <" in text
    )
    assert text.count("> ") == 2, "a reference without a quote has no block quote"


def test_a_reference_shows_its_path_its_lines_its_note_and_the_text_of_those_lines(site: Site):
    site.topic()

    site.build()

    text = site.page("structured-logging")
    assert (
        "\n### `src/app.py:3-4`\n\nthe setup\n\n```python\ndef get_logger():\n    pass\n```\n"
    ) in text


def test_a_reference_without_lines_shows_the_whole_file(site: Site):
    site.topic(references=[Ref(path="src/app.py", note="all", text=SOURCE)])

    site.build()

    text = site.page("structured-logging")
    assert "\n### `src/app.py`\n" in text
    assert f"```python\n{SOURCE}```\n" in text


def test_the_language_of_the_block_comes_from_the_extension_of_the_file(site: Site):
    site.topic(
        references=[
            Ref(path="src/lib.rs", note="a", text="fn main() {}\n"),
            Ref(path="src/notes.xyz", note="b", text="plain\n"),
        ]
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
    site.topic(references=[Ref(path="docs/guide.md", text=guide)])

    site.build()

    text = site.page("structured-logging")
    assert f"\n{fence}markdown\n{guide}{fence}\n" in text


def test_an_external_reference_shows_its_repository_its_commit_its_license_and_its_lines(
    site: Site,
):
    site.topic(
        references=[
            Ref(
                repo="github.com/acme/app",
                rev=SHA,
                license="MIT",
                path="a.py",
                lines="1-2",
                text="one\ntwo\n",
            )
        ]
    )

    site.build()

    text = site.page("structured-logging")
    link = f"https://github.com/acme/app/blob/{SHA}/a.py#L1-L2"
    head = f"\n### `a.py:1-2`\n\nn\n\ngithub.com/acme/app at {SHA} (MIT)\n\n<{link}>\n"
    assert f"{head}\n```python\none\ntwo\n```\n" in text


@pytest.mark.parametrize(
    ("repo", "lines", "fragment"),
    [
        ("github.com/acme/app", "3", "#L3>"),
        ("github.com/acme/app", "3-9", "#L3-L9>"),
        ("github.com/acme/app", None, "/a.py>"),
        ("gitlab.example.com/acme/app", "3", None),
    ],
)
def test_the_permalink_names_the_lines_and_is_written_for_github_only(
    site: Site, repo: str, lines: str | None, fragment: str | None
):
    site.topic(
        references=[
            Ref(repo=repo, rev=SHA, license="MIT", path="a.py", lines=lines, text="x\n"),
        ]
    )

    site.build()

    text = site.page("structured-logging")
    if fragment is None:
        assert "/blob/" not in text
    else:
        assert f"<https://{repo}/blob/{SHA}/a.py" in text
        assert fragment in text
        assert "#L3-L3" not in text


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
    site.topic("in-memory-adapters", title="In-memory adapters")
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
