"""What the tests of the pages of the topics share: the export of the topics and the site."""

from __future__ import annotations

import importlib.util
import json
import sys
from dataclasses import asdict, dataclass
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "site_references", Path(__file__).resolve().parents[1] / "scripts" / "site_references.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
sr = importlib.util.module_from_spec(_SPEC)
sys.modules["site_references"] = sr
_SPEC.loader.exec_module(sr)


SOURCE = "import os\n\ndef get_logger():\n    pass\n\nTAIL = 1\n"
SHA = "3f2a9c1d5e7b4a6c8d0e1f2a3b4c5d6e7f8a9b0c"
README = "Read the setup.\n\n## What to read\n\nThe function.\n"


@dataclass(frozen=True)
class Ref:
    """A reference as `botica refs export` prints it: `text` holds the lines of a file."""

    note: str = "n"
    file: str | None = None
    path: str | None = None
    url: str | None = None
    lines: str | None = None
    anchor: str | None = None
    quote: str | None = None
    repo: str | None = None
    rev: str | None = None
    license: str | None = None
    text: str | None = None


FILE = Ref(
    path="src/app.py",
    lines="3-4",
    anchor="def get_logger",
    note="the setup",
    text="def get_logger():\n    pass\n",
)


class Site:
    """The export of the topics and the `docs/` of the site."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.export = root / "topics.json"
        self.docs = root / "docs"
        self.topics: list[dict[str, object]] = []

    def topic(
        self,
        topic_id: str = "structured-logging",
        references: list[Ref] | None = None,
        title: str = "Structured logging",
    ) -> None:
        shown = [FILE] if references is None else references
        self.topics = [topic for topic in self.topics if topic["id"] != topic_id]
        self.topics.append(
            {
                "id": topic_id,
                "title": title,
                "origin": "project",
                "readme": README,
                "references": [asdict(item) for item in shown],
            }
        )

    def rule(self, family: str, slug: str, code: str, cites: str | None = None) -> Path:
        page = self.docs / "rules" / family / f"{code.lower()}-{slug}.md"
        page.parent.mkdir(parents=True, exist_ok=True)
        cited = f"references: {cites}\n" if cites else ""
        page.write_text(
            f'---\ntitle: "{slug}"\nrule: "{slug}"\ncode: "{code}"\n{cited}---\n\n# {slug}\n',
            encoding="utf-8",
        )
        return page

    def standard(self, number: int, slug: str, cites: str | None = None) -> Path:
        page = self.docs / "standards" / "CES" / f"{number}-{slug}.md"
        page.parent.mkdir(parents=True, exist_ok=True)
        cited = f"references: {cites}\n" if cites else ""
        page.write_text(
            f"---\nid: CES-{number}\ntitle: A standard\n{cited}---\n\nText.\n", encoding="utf-8"
        )
        return page

    def build(self) -> None:
        self.export.write_text(json.dumps({"topics": self.topics}), encoding="utf-8")
        sr.build(self.export, self.docs)

    def page(self, topic_id: str) -> str:
        return (self.docs / "references" / f"{topic_id}.md").read_text(encoding="utf-8")
