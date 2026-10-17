"""Add the field that filters the tables of an index page of the site.

`scripts/build_site.sh` runs it on the index of the rules, of the standards and of the topics. The
field goes above the first table of the page, or above the heading of that table, and
`docs/javascripts/filter.js` hides the rows and the sections that do not hold every word typed
in it.
"""

from __future__ import annotations

from pathlib import Path  # noqa: TC003 (cyclopts reads the annotations of `main` at run time)

import cyclopts

app = cyclopts.App(name="site-filter", help=__doc__)

FIELD = (
    '<input class="botica-filter" type="search" placeholder="Filter by code, name, tag or keyword"'
    ' aria-label="Filter the list" autocomplete="off">\n'
    '<p class="botica-filter-count" aria-live="polite"></p>\n\n'
)


def _insertion_point(lines: list[str]) -> int | None:
    """Return the index of the line the field goes above, or None when the page has no table."""
    in_code = False
    last_heading: int | None = None
    for position, line in enumerate(lines):
        if line.startswith("```"):
            in_code = not in_code
        elif in_code:
            continue
        elif line.startswith("## "):
            last_heading = position
        elif line.startswith("|"):
            return position if last_heading is None else last_heading
    return None


def add_filter(page: Path) -> None:
    """Put the field above the first table of PAGE, unless it has one."""
    text = page.read_text(encoding="utf-8")
    if 'class="botica-filter"' in text:
        return
    lines = text.splitlines(keepends=True)
    point = _insertion_point(lines)
    if point is None:
        message = f"{page.name} has no table to filter"
        raise ValueError(message)
    page.write_text("".join(lines[:point]) + FIELD + "".join(lines[point:]), encoding="utf-8")


@app.default
def main(pages: list[Path]) -> None:
    """Add the field that filters the tables of each of PAGES."""
    for page in pages:
        add_filter(page)


if __name__ == "__main__":
    app()
