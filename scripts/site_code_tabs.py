"""Put each fenced code block of the generated pages in a tab named by its language.

The READMEs of the rules and the standards are plain markdown, read on GitHub as well, so they
carry fenced blocks and no tab syntax. `scripts/build_site.sh` runs this over the copies that the
site builds: a block becomes a content tab named by its language. Tabs that follow each other with
only blank lines between them are the tabs of one group, which is how a Python example and a
TypeScript one sit side by side. A block with no language, or with one that has no name here, is
left as it is.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path  # noqa: TC003 (cyclopts reads the annotations of `main` at run time)

import cyclopts

app = cyclopts.App(name="site-code-tabs", help=__doc__)

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


@dataclass(frozen=True)
class Block:
    """A fenced block with the label of its language, from its opening fence to its closing one."""

    label: str
    lines: list[str]


Segment = list[str] | Block


def _label(opening: str) -> str | None:
    words = opening.removeprefix(FENCE).split()
    return LABELS.get(words[0].lower()) if words else None


def _segments(lines: list[str]) -> list[Segment]:
    """Split a page into runs of text and blocks that have a label."""
    segments: list[Segment] = []
    text: list[str] = []
    index = 0
    while index < len(lines):
        if not lines[index].startswith(FENCE):
            text.append(lines[index])
            index += 1
            continue
        end = index + 1
        while end < len(lines) and lines[end].rstrip() != FENCE:
            end += 1
        block = lines[index : end + 1]
        label = _label(lines[index])
        if label is None:
            text.extend(block)
        else:
            if text:
                segments.append(text)
                text = []
            segments.append(Block(label, block))
        index = end + 1
    if text:
        segments.append(text)
    return segments


def _tab(block: Block) -> list[str]:
    body = (INDENT + line if line else "" for line in block.lines)
    return [f'=== "{block.label}"', "", *body]


def tabbed(text: str) -> str:
    """Return `text` with each block that has a language in a tab."""
    out: list[str] = []
    for segment in _segments(text.splitlines()):
        out += _tab(segment) if isinstance(segment, Block) else segment
    return "\n".join(out) + ("\n" if text.endswith("\n") else "")


def apply(roots: list[Path]) -> None:
    """Rewrite every page under each of `roots`."""
    for root in roots:
        for page in sorted(root.rglob("*.md")):
            page.write_text(tabbed(page.read_text(encoding="utf-8")), encoding="utf-8")


@app.default
def main(*roots: Path) -> None:
    """Put the code blocks of the pages under ROOTS in tabs."""
    apply(list(roots))


if __name__ == "__main__":
    app()
