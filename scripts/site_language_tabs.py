"""Show what depends on the language in a tab for each language.

The READMEs of the rules and the standards are plain markdown, read on GitHub as well, so they
carry no tab syntax. `scripts/build_site.sh` runs this over the copies that the site builds, and two
things become content tabs:

- A run of `### <Language>` subsections, for example `### Python` and `### TypeScript`. Each
  subsection, prose and code, is the content of one tab, and it ends at the next heading of level
  three or above.
- A fenced block with a language that is outside such a subsection.

Tabs that follow each other are the tabs of one group, and the site keeps the language chosen in
all of them. A block with no language, or with one that has no name here, is left as it is.
"""

from __future__ import annotations

import re
from pathlib import Path  # noqa: TC003 (cyclopts reads the annotations of `main` at run time)

import cyclopts

app = cyclopts.App(name="site-language-tabs", help=__doc__)

LABELS = {
    "bash": "Bash",
    "sh": "Bash",
    "json": "JSON",
    "js": "JavaScript",
    "javascript": "JavaScript",
    "py": "Python",
    "python": "Python",
    "rust": "Rust",
    "toml": "TOML",
    "ts": "TypeScript",
    "typescript": "TypeScript",
    "yaml": "YAML",
    "yml": "YAML",
}
FENCE = "```"
INDENT = "    "
HEADING = re.compile(r"^(#{1,6}) +(.+?) *$")
SECTION_LEVEL = 3


def _block_label(opening: str) -> str | None:
    words = opening.removeprefix(FENCE).split()
    return LABELS.get(words[0].lower()) if words else None


def _fence_end(lines: list[str], start: int) -> int:
    """Return the index of the line that closes the block opened at `start`."""
    end = start + 1
    while end < len(lines) and lines[end].rstrip() != FENCE:
        end += 1
    return min(end, len(lines) - 1)


def _heading_level(line: str) -> int:
    found = HEADING.match(line)
    return len(found.group(1)) if found else 0


def _language_of(line: str) -> str | None:
    found = HEADING.match(line)
    if found and len(found.group(1)) == SECTION_LEVEL and found.group(2) in LABELS.values():
        return found.group(2)
    return None


def _trim(lines: list[str]) -> list[str]:
    start, end = 0, len(lines)
    while start < end and not lines[start].strip():
        start += 1
    while end > start and not lines[end - 1].strip():
        end -= 1
    return lines[start:end]


def _section_body(lines: list[str], start: int) -> tuple[list[str], int]:
    """Take the lines of a subsection, a block at a time so that a `#` in code is not a heading."""
    body: list[str] = []
    index = start
    while index < len(lines):
        if lines[index].startswith(FENCE):
            end = _fence_end(lines, index)
            body += lines[index : end + 1]
            index = end + 1
        elif 0 < _heading_level(lines[index]) <= SECTION_LEVEL:
            break
        else:
            body.append(lines[index])
            index += 1
    return _trim(body), index


def _tab(label: str, body: list[str]) -> list[str]:
    return [f'=== "{label}"', "", *(INDENT + line if line else "" for line in body)]


def _group(lines: list[str], start: int) -> tuple[list[str], int]:
    """Turn the run of language subsections that starts at `start` into the tabs of one group."""
    tabs: list[list[str]] = []
    index = start
    while index < len(lines) and (label := _language_of(lines[index])):
        body, index = _section_body(lines, index + 1)
        tabs.append(_tab(label, body))
    rendered: list[str] = []
    for position, tab in enumerate(tabs):
        rendered += ([""] if position else []) + tab
    return rendered + ([""] if index < len(lines) else []), index


def tabbed(text: str) -> str:
    """Return `text` with its language subsections and its blocks that have a language in tabs."""
    lines = text.splitlines()
    out: list[str] = []
    index = 0
    while index < len(lines):
        if _language_of(lines[index]):
            group, index = _group(lines, index)
            out += group
        elif lines[index].startswith(FENCE):
            end = _fence_end(lines, index)
            block = lines[index : end + 1]
            label = _block_label(lines[index])
            out += _tab(label, block) if label else block
            index = end + 1
        else:
            out.append(lines[index])
            index += 1
    return "\n".join(out) + ("\n" if text.endswith("\n") else "")


def apply(roots: list[Path]) -> None:
    """Rewrite every page under each of `roots`."""
    for root in roots:
        for page in sorted(root.rglob("*.md")):
            page.write_text(tabbed(page.read_text(encoding="utf-8")), encoding="utf-8")


@app.default
def main(*roots: Path) -> None:
    """Put what depends on the language on the pages under ROOTS in tabs."""
    apply(list(roots))


if __name__ == "__main__":
    app()
