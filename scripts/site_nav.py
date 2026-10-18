"""Write the configuration of the site with the navigation that its pages call for.

Zensical orders the pages of a folder by name, which puts `references` and `rules` before
`standards`, and its `nav` takes pages and not folders. The pages of the rules, of the standards and
of the topics of references are made by `scripts/build_site.sh`, so the list is made here, from
what the folders hold, and added to the configuration that the build reads. The topics of references
are not listed: the index of references links each one. The titles of the pages are left to
Zensical.
"""

from __future__ import annotations

import json
import re
from pathlib import Path  # noqa: TC003 (cyclopts reads the annotations of `main` at run time)
from typing import TYPE_CHECKING

import cyclopts

if TYPE_CHECKING:
    from collections.abc import Sequence

app = cyclopts.App(name="site-nav", help=__doc__)

NAMES = {"fastapi": "FastAPI", "botica": "Botica"}
LEADING_NUMBER = re.compile(r"^(\d+)-")


def _title(folder: str) -> str:
    return NAMES.get(folder, folder.replace("-", " ").capitalize())


def _order(page: Path) -> tuple[int, int, str]:
    """Put the index first, then the standards by number and the rules by name."""
    if page.name == "index.md":
        return (0, 0, "")
    number = LEADING_NUMBER.match(page.name)
    return (1, int(number.group(1)) if number else 0, page.name)


def _pages(folder: Path, docs: Path) -> list[str]:
    return [page.relative_to(docs).as_posix() for page in sorted(folder.glob("*.md"), key=_order)]


def _section(title: str, entries: Sequence[object]) -> dict[str, object]:
    return {title: entries}


def _folders(root: Path) -> list[Path]:
    return sorted(path for path in root.iterdir() if path.is_dir())


def navigation(docs: Path) -> list[object]:
    """Return the `nav` of the site: the home, the standards, the rules and the references."""
    nav: list[object] = [{"Botica": "index.md"}]
    if (docs / "use.md").is_file():
        nav.append({"Use": "use.md"})
    standards = docs / "standards"
    if standards.is_dir():
        sets = [_section(path.name, _pages(path, docs)) for path in _folders(standards)]
        nav.append(_section("Standards", ["standards/index.md", *sets]))
    rules = docs / "rules"
    if rules.is_dir():
        families = [_section(_title(path.name), _pages(path, docs)) for path in _folders(rules)]
        nav.append(_section("Rules", ["rules/index.md", *families]))
    if (docs / "references" / "index.md").is_file():
        nav.append({"References": "references/index.md"})
    return nav


def _toml(value: object) -> str:
    """Write a value as TOML: a JSON string is a TOML basic string, and the rest is inline."""
    if isinstance(value, dict):
        return (
            "{ "
            + ", ".join(f"{json.dumps(key)} = {_toml(item)}" for key, item in value.items())
            + " }"
        )
    if isinstance(value, list):
        return "[" + ", ".join(_toml(item) for item in value) + "]"
    return json.dumps(value)


def configuration(base: str, docs: Path) -> str:
    """Return the configuration `base` with the navigation of the pages of `docs`."""
    head = "[project]\n"
    if not base.startswith(head):
        raise ValueError("the configuration must start with the [project] table")
    return f"{head}nav = {_toml(navigation(docs))}\n{base[len(head) :]}"


@app.default
def main(base: Path, docs: Path, output: Path) -> None:
    """Write OUTPUT: the configuration BASE with the navigation of the pages of DOCS."""
    output.write_text(configuration(base.read_text(encoding="utf-8"), docs), encoding="utf-8")


if __name__ == "__main__":
    app()
