"""The navigation of the site lists the standards, the rules and the topics of references/.

Asserted through the public seam of `scripts/site_nav.py`: the folders of the pages go in, the
`nav` of the site comes out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "site_nav", Path(__file__).resolve().parents[1] / "scripts" / "site_nav.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
sn = importlib.util.module_from_spec(_SPEC)
sys.modules["site_nav"] = sn
_SPEC.loader.exec_module(sn)


def page(docs: Path, relative: str) -> None:
    path = docs / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("---\ntitle: T\n---\n", encoding="utf-8")


def test_the_references_come_last_with_their_index_first_and_the_topics_by_name(tmp_path: Path):
    for relative in (
        "index.md",
        "standards/index.md",
        "rules/index.md",
        "references/index.md",
        "references/structured-logging.md",
        "references/in-memory-adapters.md",
    ):
        page(tmp_path, relative)

    nav = sn.navigation(tmp_path)

    assert nav[-1] == {
        "References": [
            "references/index.md",
            "references/in-memory-adapters.md",
            "references/structured-logging.md",
        ]
    }
    assert [next(iter(entry)) for entry in nav] == ["Botica", "Standards", "Rules", "References"]


def test_a_site_without_topics_has_no_references_in_its_navigation(tmp_path: Path):
    page(tmp_path, "index.md")
    page(tmp_path, "rules/index.md")

    assert "References" not in [next(iter(entry)) for entry in sn.navigation(tmp_path)]
