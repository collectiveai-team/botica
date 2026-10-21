"""Write the llms.txt of the site from the manual and the pages of the rules, standards and topics.

`scripts/build_site.sh` runs it after Zensical has built the site. The name, the description and
the address of the site come from the `[project]` table of the configuration.
"""

from __future__ import annotations

import json
import tomllib
from dataclasses import dataclass, fields
from pathlib import Path  # noqa: TC003 (cyclopts reads the annotations at runtime)

import cyclopts

app = cyclopts.App(name="site-llms", help=__doc__)


@dataclass(frozen=True)
class FrontMatter:
    """The fields of the front matter of a page that llms.txt shows; a missing one is empty."""

    rule: str = ""
    code: str = ""
    description: str = ""
    id: str = ""
    title: str = ""
    subtitle: str = ""


def _front_matter(page: Path) -> FrontMatter:
    """Return the front matter of a page, with the quotes of its values removed."""
    head, _, _ = page.read_text(encoding="utf-8").removeprefix("---\n").partition("\n---\n")
    known = {field.name for field in fields(FrontMatter)}
    values = {}
    for key, _, raw in (line.partition(": ") for line in head.splitlines()):
        if raw and key in known:
            values[key] = json.loads(raw) if raw.startswith('"') else raw
    return FrontMatter(**values)


def _address(site: str, docs: Path, page: Path) -> str:
    return site + page.relative_to(docs).with_suffix("").as_posix() + "/"


def _rules(site: str, docs: Path) -> str:
    pages = sorted((docs / "rules").glob("*/*.md"), key=lambda page: _front_matter(page).code)
    items = []
    for page in pages:
        front = _front_matter(page)
        name = f"{front.rule} ({front.code})"
        items.append(f"- [{name}]({_address(site, docs, page)}): {front.description}\n")
    return "\n## Rules\n\n" + "".join(items)


def _number(page: Path) -> tuple[str, int]:
    prefix, _, number = _front_matter(page).id.rpartition("-")
    return prefix, int(number)


def _standards(site: str, docs: Path) -> str:
    pages = [page for page in (docs / "standards").glob("*/*.md") if page.name != "index.md"]
    items = []
    for page in sorted(pages, key=_number):
        front = _front_matter(page)
        items.append(f"- [{front.id}]({_address(site, docs, page)}): {front.title}\n")
    return "\n## Standards\n\n" + "".join(items)


def _manual(site: str, docs: Path) -> str:
    pages = [
        *sorted(docs.glob("start.md")),
        *sorted(docs.glob("guide/*.md")),
        *sorted(docs.glob("extend/*.md")),
    ]
    if not pages:
        return ""
    items = [f"- [{_front_matter(page).title}]({_address(site, docs, page)})\n" for page in pages]
    return "\n## Manual\n\n" + "".join(items)


def _references(site: str, docs: Path) -> str:
    pages = sorted(page for page in (docs / "references").glob("*.md") if page.name != "index.md")
    if not pages:
        return ""
    items = []
    for page in pages:
        front = _front_matter(page)
        items.append(f"- [{front.subtitle}]({_address(site, docs, page)}): {front.title}\n")
    return "\n## References\n\n" + "".join(items)


def write(config: Path, docs: Path, output: Path) -> None:
    """Write the llms.txt of the site configured in CONFIG, with the pages under DOCS, to OUTPUT."""
    project = tomllib.loads(config.read_text(encoding="utf-8"))["project"]
    text = f"# {project['site_name']}\n\n> {project['site_description']}\n"
    site = project["site_url"]
    text += _manual(site, docs) + _rules(site, docs) + _standards(site, docs)
    text += _references(site, docs)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")


@app.default
def main(config: Path, docs: Path, output: Path) -> None:
    """Write OUTPUT, the llms.txt of the site configured in CONFIG, from the pages under DOCS."""
    write(config, docs, output)


if __name__ == "__main__":
    app()
