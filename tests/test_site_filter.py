"""The index pages of the site get a field that filters their tables.

Asserted through the public seam of `scripts/site_filter.py`: a page goes in and the page with the
field comes out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

_SPEC = importlib.util.spec_from_file_location(
    "site_filter", Path(__file__).resolve().parents[1] / "scripts" / "site_filter.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
sf = importlib.util.module_from_spec(_SPEC)
sys.modules["site_filter"] = sf
_SPEC.loader.exec_module(sf)

FIELD = (
    '<input class="botica-filter" type="search" placeholder="Filter by code, name, tag or keyword"'
    ' aria-label="Filter the list" autocomplete="off">\n'
    '<p class="botica-filter-count" aria-live="polite"></p>\n\n'
)
TABLE = "| A | B |\n|---|---|\n| 1 | 2 |\n"


def page(tmp_path: Path, text: str) -> Path:
    path = tmp_path / "index.md"
    path.write_text(text, encoding="utf-8")
    return path


def test_the_field_goes_above_the_first_table_of_a_page_without_headings(tmp_path: Path):
    path = page(tmp_path, f"---\ntitle: T\n---\n\n# T\n\nIntro.\n\n{TABLE}")

    sf.add_filter(path)

    assert path.read_text(encoding="utf-8") == (
        f"---\ntitle: T\n---\n\n# T\n\nIntro.\n\n{FIELD}{TABLE}"
    )


def test_the_field_goes_above_the_heading_of_the_first_table(tmp_path: Path):
    before = "# Rules\n\nIntro.\n\n## Sets\n\n- a list\n\n## architecture\n\nWhat it is.\n\n"
    path = page(tmp_path, f"{before}{TABLE}\n## botica\n\n{TABLE}")

    sf.add_filter(path)

    expected = (
        f"# Rules\n\nIntro.\n\n## Sets\n\n- a list\n\n{FIELD}## architecture\n\nWhat it is.\n\n"
    )
    assert path.read_text(encoding="utf-8") == f"{expected}{TABLE}\n## botica\n\n{TABLE}"


def test_a_page_that_has_the_field_does_not_get_a_second_one(tmp_path: Path):
    path = page(tmp_path, f"# T\n\n{TABLE}")
    sf.add_filter(path)
    once = path.read_text(encoding="utf-8")

    sf.add_filter(path)

    assert path.read_text(encoding="utf-8") == once


def test_a_page_without_a_table_is_an_error_that_names_it(tmp_path: Path):
    path = page(tmp_path, "# T\n\nNothing to filter.\n")

    with pytest.raises(ValueError, match=r"index\.md has no table"):
        sf.add_filter(path)


def test_a_table_inside_a_block_of_code_is_not_a_table(tmp_path: Path):
    path = page(tmp_path, f"# T\n\n```markdown\n{TABLE}```\n\n## Real\n\n{TABLE}")

    sf.add_filter(path)

    text = path.read_text(encoding="utf-8")
    assert text.index("botica-filter") > text.index("```\n\n")
    assert text.index("botica-filter") < text.index("## Real")


def test_a_line_that_starts_with_inline_code_does_not_open_a_block_of_code(tmp_path: Path):
    path = page(tmp_path, f"# T\n\n``x`` is inline code.\n\n{TABLE}")

    sf.add_filter(path)

    assert path.read_text(encoding="utf-8") == f"# T\n\n``x`` is inline code.\n\n{FIELD}{TABLE}"


def test_the_script_that_does_the_filtering_exists_and_the_site_loads_it():
    site = Path(__file__).resolve().parents[1] / "website"

    configuration = (site / "zensical.toml").read_text(encoding="utf-8")

    assert 'extra_javascript = ["javascripts/filter.js"]' in configuration
    script = (site / "docs" / "javascripts" / "filter.js").read_text(encoding="utf-8")
    assert "input.botica-filter" in script
