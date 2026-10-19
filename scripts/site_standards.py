"""Relate the pages of the standards and the pages of the rules on the site.

`scripts/build_site.sh` copies `standards/` into the documents of the site after botica has
exported the pages of the rules. The relation is made here, from the `standard` that the front
matter of each rule page carries: the page of a standard gets its id as subtitle and the list of
its rules at the end, and the page of a rule links the standard it names. The index of the standards
gets a table of all of them, with their rules and the topics they cite. The text of the standard in
`standards/` is never touched.
"""

from __future__ import annotations

import json
import os
from collections import defaultdict
from dataclasses import dataclass, field
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


def _value(raw: str) -> str | list[str]:
    return json.loads(raw) if raw.startswith(('"', "[")) else raw


@dataclass(frozen=True)
class FrontMatter:
    """The fields of the front matter of a page that this script reads; a missing one is empty."""

    id: str = ""
    title: str = ""
    rule: str = ""
    code: str = ""
    standard: str = ""
    references: list[str] = field(default_factory=list)


RulesTable = dict[str, list[RuleLink]]
PagesTable = dict[str, Path]


def _front_matter(lines: list[str]) -> FrontMatter:
    values = {key: _value(raw) for key, _, raw in (line.partition(": ") for line in lines) if raw}
    references = values.get("references", [])
    return FrontMatter(
        id=str(values.get("id", "")),
        title=str(values.get("title", "")),
        rule=str(values.get("rule", "")),
        code=str(values.get("code", "")),
        standard=str(values.get("standard", "")),
        references=[str(item) for item in references] if isinstance(references, list) else [],
    )


def _rules_by_standard(docs: Path) -> RulesTable:
    found: RulesTable = defaultdict(list)
    for page in sorted((docs / "rules").glob("*/*.md")):
        lines, _ = _split(page.read_text(encoding="utf-8"))
        front = _front_matter(lines)
        if front.standard:
            found[front.standard].append(RuleLink(front.rule, front.code, page))
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


def _decorated(page: Path, rules: RulesTable) -> str:
    lines, body = _split(page.read_text(encoding="utf-8"))
    standard = _front_matter(lines).id
    lines = [*lines, f"subtitle: {standard}"]
    text = "---\n" + "\n".join(lines) + FRONT_MATTER_END + body
    if standard in rules:
        text += _rule_list(page, rules[standard])
    return text


def _standard_pages(docs: Path) -> PagesTable:
    pages: PagesTable = {}
    for page in sorted((docs / "standards").glob("*/*.md")):
        if page.name != "index.md":
            lines, _ = _split(page.read_text(encoding="utf-8"))
            pages[_front_matter(lines).id] = page
    return pages


def _link_standard(rule: RuleLink, standard: str, target: Path) -> None:
    address = Path(os.path.relpath(target, rule.page.parent)).as_posix()
    text = rule.page.read_text(encoding="utf-8")
    plain, linked = f"Standard: {standard}", f"Standard: [{standard}]({address})"
    rule.page.write_text(text.replace(plain, linked, 1), encoding="utf-8")


def _number(standard: str) -> tuple[str, int]:
    """Return the set and the number of an id such as `CES-71`, to sort by number."""
    prefix, _, number = standard.rpartition("-")
    return prefix, int(number)


def _cell(text: str) -> str:
    return text.replace("|", "\\|")


def _standards_table(index: Path, standards: PagesTable, rules: RulesTable) -> str:
    rows = []
    for standard in sorted(standards, key=_number):
        page = standards[standard]
        front = _front_matter(_split(page.read_text(encoding="utf-8"))[0])
        address = Path(os.path.relpath(page, index.parent)).as_posix()
        names = ", ".join(
            f"[`{link.slug}`]({Path(os.path.relpath(link.page, index.parent)).as_posix()})"
            for link in rules.get(standard, [])
        )
        topics = ", ".join(f"[{topic}](../references/{topic}.md)" for topic in front.references)
        title = _cell(front.title)
        rows.append(f"| [{standard}]({address}) | {title} | {names} | {topics} |\n")
    header = "| Standard | Title | Rules | Topics |\n|---|---|---|---|\n"
    return "\n## All standards\n\n" + header + "".join(rows)


def decorate(docs: Path) -> None:
    """Rewrite the standards under `docs/standards` and the rules that name them."""
    rules = _rules_by_standard(docs)
    standards = _standard_pages(docs)
    index = docs / "standards" / "index.md"
    if index.is_file():
        text = index.read_text(encoding="utf-8")
        index.write_text(text + _standards_table(index, standards, rules), encoding="utf-8")
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
