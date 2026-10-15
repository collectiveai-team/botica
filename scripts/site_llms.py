"""Write the llms.txt of the site from the pages of the rules and the standards.

`scripts/build_site.sh` runs it after Zensical has built the site. The name, the description and
the address of the site come from the `[project]` table of the configuration.
"""

from __future__ import annotations

import json
import tomllib
from pathlib import Path  # noqa: TC003 (cyclopts reads the annotations at runtime)

import cyclopts

app = cyclopts.App(name="site-llms", help=__doc__)


def _fields(page: Path) -> dict[str, str]:
    """Return the fields of the front matter of a page, with their quotes removed."""
    head, _, _ = page.read_text(encoding="utf-8").removeprefix("---\n").partition("\n---\n")
    pairs = (line.partition(": ") for line in head.splitlines())
    return {key: json.loads(raw) if raw.startswith('"') else raw for key, _, raw in pairs if raw}


def _address(site: str, docs: Path, page: Path) -> str:
    return site + page.relative_to(docs).with_suffix("").as_posix() + "/"


def _rules(site: str, docs: Path) -> str:
    pages = sorted((docs / "rules").glob("*/*.md"), key=lambda page: _fields(page)["code"])
    items = []
    for page in pages:
        fields = _fields(page)
        name = f"{fields['rule']} ({fields['code']})"
        items.append(f"- [{name}]({_address(site, docs, page)}): {fields['description']}\n")
    return "\n## Rules\n\n" + "".join(items)


def _number(page: Path) -> tuple[str, int]:
    prefix, _, number = _fields(page)["id"].rpartition("-")
    return prefix, int(number)


def _standards(site: str, docs: Path) -> str:
    pages = [page for page in (docs / "standards").glob("*/*.md") if page.name != "index.md"]
    items = []
    for page in sorted(pages, key=_number):
        fields = _fields(page)
        items.append(f"- [{fields['id']}]({_address(site, docs, page)}): {fields['title']}\n")
    return "\n## Standards\n\n" + "".join(items)


def write(config: Path, docs: Path, output: Path) -> None:
    """Write the llms.txt of the site configured in CONFIG, with the pages under DOCS, to OUTPUT."""
    project = tomllib.loads(config.read_text(encoding="utf-8"))["project"]
    text = f"# {project['site_name']}\n\n> {project['site_description']}\n"
    text += _rules(project["site_url"], docs) + _standards(project["site_url"], docs)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(text, encoding="utf-8")


@app.default
def main(config: Path, docs: Path, output: Path) -> None:
    """Write OUTPUT, the llms.txt of the site configured in CONFIG, from the pages under DOCS."""
    write(config, docs, output)


if __name__ == "__main__":
    app()
