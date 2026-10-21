"""The navigation of the site lists the standards, the rules and the index of references/.

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


def test_the_references_come_last_as_their_index_alone_and_the_topics_are_left_out(tmp_path: Path):
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

    assert nav[-1] == {"References": "references/index.md"}
    assert [next(iter(entry)) for entry in nav] == ["Botica", "Standards", "Rules", "References"]


def test_the_manual_comes_before_the_standards_with_its_pages_in_the_order_of_reading(
    tmp_path: Path,
):
    for relative in (
        "index.md",
        "start.md",
        "guide/refs.md",
        "guide/lint.md",
        "guide/zebra.md",
        "guide/configuration.md",
        "extend/extensions.md",
        "extend/catalogs.md",
        "standards/index.md",
    ):
        page(tmp_path, relative)

    nav = sn.navigation(tmp_path)

    assert nav[:4] == [
        {"Botica": "index.md"},
        {"Start": "start.md"},
        {"Guide": ["guide/lint.md", "guide/configuration.md", "guide/refs.md", "guide/zebra.md"]},
        {"Extend": ["extend/catalogs.md", "extend/extensions.md"]},
    ]
    assert next(iter(nav[4])) == "Standards"


def test_a_site_without_topics_has_no_references_in_its_navigation(tmp_path: Path):
    page(tmp_path, "index.md")
    page(tmp_path, "rules/index.md")

    assert "References" not in [next(iter(entry)) for entry in sn.navigation(tmp_path)]
