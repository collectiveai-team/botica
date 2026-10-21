"""The index of the topics lists each one with what it points at and who cites it.

Asserted through the public seam of `scripts/site_references.py`: the export of the topics and the
pages of the rules and the standards go in, and the index of the topics comes out (CES-65).
"""

from __future__ import annotations

from site_references_support import SHA, Ref, Site

HEADER = "| Topic | Points at | Cited by |\n|---|---|---|\n"


def index(site: Site) -> str:
    return (site.docs / "references" / "index.md").read_text(encoding="utf-8")


def rows(site: Site) -> list[str]:
    text = index(site)
    assert HEADER in text
    return text.split(HEADER, 1)[1].splitlines()


def test_the_index_starts_with_its_title_and_an_introduction(site: Site):
    site.topic()

    site.build()

    assert index(site).startswith("---\ntitle: References\n---\n\n# References\n\n")


def test_a_topic_is_a_row_with_its_title_what_it_points_at_and_nothing_citing_it(site: Site):
    site.topic(
        "structured-logging",
        references=[
            Ref(path="src/app.py", lines="3-4"),
            Ref(url="https://example.com/docs#setup"),
            Ref(path="a.py", repo="github.com/acme/app", rev=SHA, license="MIT"),
        ],
    )

    site.build()

    assert rows(site) == [
        "| [structured-logging](structured-logging.md)<br>Structured logging | "
        "`src/app.py:3-4`<br>https://example.com/docs#setup<br>`a.py` (github.com/acme/app) |  |"
    ]


def test_the_topics_are_rows_by_id(site: Site):
    site.topic("structured-logging")
    site.topic("in-memory-adapters", title="In-memory adapters")

    site.build()

    ids = [row.split("]")[0].removeprefix("| [") for row in rows(site)]
    assert ids == ["in-memory-adapters", "structured-logging"]


def test_the_rules_and_the_standards_that_cite_a_topic_are_in_its_row_with_a_link_each(site: Site):
    site.topic("structured-logging")
    site.topic("in-memory-adapters", title="In-memory adapters")
    site.rule("logging", "log-no-print", "BTLOG002", '["structured-logging"]')
    site.standard(74, "core-logger", '["structured-logging"]')

    site.build()

    uncited, row = rows(site)
    assert uncited.endswith("|  |"), "a topic that nothing cites has an empty cell"
    cited = row.rsplit("|", 2)[1].strip()
    assert cited == (
        "[`log-no-print`](../rules/logging/btlog002-log-no-print.md) (BTLOG002), "
        "[CES-74](../standards/CES/74-core-logger.md)"
    )


def test_a_pipe_in_a_title_does_not_split_its_cell(site: Site):
    site.topic(title="a | b")

    site.build()

    assert "<br>a \\| b |" in rows(site)[0]
