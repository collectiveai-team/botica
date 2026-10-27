"""Give the proposed standard of a set its number, as the last commit before the merge.

A proposal carries its standard as `standards/<set>/0-<slug>.md`, with `id: <SET>-0`, and its rules
name it as `standard = "<SET>-0"` (ADR 0004). Run on the branch rebased onto `main`, this takes the
next free number of the set, the highest one plus one, renames the file and writes the number
wherever `<SET>-0` is written: its `id`, the `standard` of its rules, and the recipes, topics and
documentation that cite it.
"""

from __future__ import annotations

import re
from pathlib import Path

import cyclopts

app = cyclopts.App(name="number-standard", help=__doc__)

STANDARD_FILE = re.compile(r"^(?P<number>\d+)-(?P<slug>.+)\.md$")
# Where a change cites its standard: the rules, recipes and topics, the standards, and the docs.
# An `index.md` describes the convention and writes `<SET>-0` as an example, so it is left alone.
CITING_FOLDERS = ("catalog", "standards", "docs", "website/docs")
CITING_FILES = ("README.md", "AGENTS.md")


def _numbered(folder: Path) -> list[tuple[int, Path]]:
    found = []
    for path in sorted(folder.glob("*.md")):
        match = STANDARD_FILE.match(path.name)
        if match:
            found.append((int(match["number"]), path))
    return found


def _proposed(folder: Path, standards: list[tuple[int, Path]]) -> Path:
    proposed = [path for value, path in standards if value == 0]
    if not proposed:
        raise SystemExit(f"{folder}: no standard numbered 0")
    if len(proposed) > 1:
        raise SystemExit(f"{folder}: more than one standard numbered 0")
    return proposed[0]


def _replace(path: Path, placeholder: re.Pattern[str], new_id: str, target: Path) -> None:
    target.write_text(placeholder.sub(new_id, path.read_text(encoding="utf-8")), encoding="utf-8")
    if target != path:
        path.unlink()


def _text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        return None


def _files_that_name(root: Path, placeholder: re.Pattern[str]) -> list[Path]:
    candidates = [root / name for name in CITING_FILES]
    for folder in CITING_FOLDERS:
        candidates.extend(path for path in (root / folder).rglob("*") if path.is_file())
    return sorted(
        path
        for path in candidates
        if path.is_file()
        and path.name != "index.md"
        and (text := _text(path)) is not None
        and placeholder.search(text)
    )


def number(root: Path, standard_set: str) -> str:
    """Give the proposed standard of `standard_set` its number and return its new id."""
    folder = root / "standards" / standard_set
    standards = _numbered(folder)
    source = _proposed(folder, standards)
    new = max(value for value, _ in standards) + 1
    new_id = f"{standard_set}-{new}"
    placeholder = re.compile(rf"\b{re.escape(standard_set)}-0\b")
    citing = [path for path in _files_that_name(root, placeholder) if path != source]
    _replace(source, placeholder, new_id, folder / f"{new}-{source.name.removeprefix('0-')}")
    for path in citing:
        _replace(path, placeholder, new_id, path)
    return new_id


@app.default
def main(standard_set: str = "CES") -> None:
    """Give the proposed standard of STANDARD_SET its number in the current checkout."""
    print(number(Path.cwd(), standard_set))


if __name__ == "__main__":
    app()
