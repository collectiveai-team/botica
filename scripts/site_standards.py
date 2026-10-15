"""Relate the pages of the standards and the pages of the rules on the site.

`scripts/build_site.sh` copies `standards/` into the documents of the site after botica has
exported the pages of the rules. The relation is made here, from the `standard` that the front
matter of each rule page carries: the page of a standard gets its id as subtitle and the list of
its rules at the end, and the page of a rule links the standard it names. The text of the standard
in `standards/` is never touched.
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


def _standard_pages(docs: Path) -> dict[str, Path]:
    pages = {}
    for page in sorted((docs / "standards").glob("*/*.md")):
        if page.name != "index.md":
            lines, _ = _split(page.read_text(encoding="utf-8"))
            pages[_fields(lines)["id"]] = page
    return pages


def _link_standard(rule: RuleLink, standard: str, target: Path) -> None:
    address = Path(os.path.relpath(target, rule.page.parent)).as_posix()
    text = rule.page.read_text(encoding="utf-8")
    plain, linked = f"Standard: {standard}", f"Standard: [{standard}]({address})"
    rule.page.write_text(text.replace(plain, linked, 1), encoding="utf-8")


def decorate(docs: Path) -> None:
    """Rewrite the standards under `docs/standards` and the rules that name them."""
    rules = _rules_by_standard(docs)
    standards = _standard_pages(docs)
    for page in standards.values():
        page.write_text(_decorated(page, rules), encoding="utf-8")
    for standard, links in rules.items():
        for link in links:
            if standard in standards:
                _link_standard(link, standard, standards[standard])


@app.default
def main(docs: Path) -> None:
    """Subtitle the standards copied under DOCS and list the rules that implement each."""
    decorate(docs)


if __name__ == "__main__":
    app()
