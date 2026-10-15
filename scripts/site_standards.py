"""Subtitle the page of each standard with its id and list the rules that implement it.

`scripts/build_site.sh` copies `standards/` into the documents of the site after botica has
exported the pages of the rules. Botica knows nothing about standards, so the list is made here,
from the `standard` that the front matter of each rule page carries, and written at the end of the
copy of the standard. The text of the standard in `standards/` is never touched.
"""

from __future__ import annotations

import json
import os
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import cyclopts

app = cyclopts.App(name="site-standards", help=__doc__)

FRONT_MATTER_END = "\n---\n"


@dataclass(frozen=True)
class RuleLink:
    """A rule page that names a standard."""

    slug: str
    code: str
    page: Path


def _split(text: str) -> tuple[list[str], str]:
    """Return the lines of the front matter of a page, and what follows it."""
    head, _, body = text.removeprefix("---\n").partition(FRONT_MATTER_END)
    return head.splitlines(), body


def _value(raw: str) -> str:
    return json.loads(raw) if raw.startswith('"') else raw


def _fields(lines: list[str]) -> dict[str, str]:
    pairs = (line.partition(": ") for line in lines)
    return {key: _value(value) for key, _, value in pairs if value}


def _rules_by_standard(docs: Path) -> dict[str, list[RuleLink]]:
    found: dict[str, list[RuleLink]] = defaultdict(list)
    for page in sorted((docs / "rules").glob("*/*.md")):
        lines, _ = _split(page.read_text(encoding="utf-8"))
        fields = _fields(lines)
        if "standard" in fields:
            found[fields["standard"]].append(RuleLink(fields["rule"], fields["code"], page))
    return {
        standard: sorted(links, key=lambda link: link.code) for standard, links in found.items()
    }


def _rule_list(page: Path, links: list[RuleLink]) -> str:
    items = (
        f"- [`{link.slug}`]({Path(os.path.relpath(link.page, page.parent)).as_posix()})"
        f" ({link.code})\n"
        for link in links
    )
    return "\n## Rules\n\n" + "".join(items)


def _decorated(page: Path, rules: dict[str, list[RuleLink]]) -> str:
    lines, body = _split(page.read_text(encoding="utf-8"))
    standard = _fields(lines)["id"]
    lines = [*lines, f"subtitle: {standard}"]
    text = "---\n" + "\n".join(lines) + FRONT_MATTER_END + body
    if standard in rules:
        text += _rule_list(page, rules[standard])
    return text


def decorate(docs: Path) -> None:
    """Rewrite the copy of each standard under `docs/standards` with its subtitle and its rules."""
    rules = _rules_by_standard(docs)
    for page in sorted((docs / "standards").glob("*/*.md")):
        if page.name != "index.md":
            page.write_text(_decorated(page, rules), encoding="utf-8")


@app.default
def main(docs: Path) -> None:
    """Subtitle the standards copied under DOCS and list the rules that implement each."""
    decorate(docs)


if __name__ == "__main__":
    app()
