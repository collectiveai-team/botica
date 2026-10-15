"""A code block on the site sits in a tab named by its language.

The READMEs of the rules and the standards are plain markdown, read on GitHub as well. The build
turns their fenced blocks into content tabs: blocks of different languages that follow each other
become the tabs of one group. Asserted through the public seam of `scripts/site_code_tabs.py`: the
text of a page goes in, the text with tabs comes out (CES-65).
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "site_code_tabs", Path(__file__).resolve().parents[1] / "scripts" / "site_code_tabs.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
sct = importlib.util.module_from_spec(_SPEC)
sys.modules["site_code_tabs"] = sct
_SPEC.loader.exec_module(sct)


def test_a_block_sits_in_a_tab_named_by_its_language():
    text = "Intro.\n\n```python\nx = 1\n```\n\nAfter.\n"

    assert sct.tabbed(text) == (
        'Intro.\n\n=== "Python"\n\n    ```python\n    x = 1\n    ```\n\nAfter.\n'
    )


def test_blocks_of_different_languages_that_follow_each_other_are_the_tabs_of_one_group():
    text = "```python\nx = 1\n```\n\n```ts\nconst x = 1;\n```\n"

    assert sct.tabbed(text) == (
        '=== "Python"\n\n    ```python\n    x = 1\n    ```\n\n'
        '=== "TypeScript"\n\n    ```ts\n    const x = 1;\n    ```\n'
    )


def test_a_paragraph_between_two_blocks_makes_two_groups():
    text = "```python\nx = 1\n```\n\nThen:\n\n```ts\nconst x = 1;\n```\n"

    assert sct.tabbed(text) == (
        '=== "Python"\n\n    ```python\n    x = 1\n    ```\n\nThen:\n\n'
        '=== "TypeScript"\n\n    ```ts\n    const x = 1;\n    ```\n'
    )


def test_a_block_with_no_language_or_an_unnamed_one_is_left_as_it_is():
    text = "```\nplain\n```\n\n```text\nlayout\n```\n"

    assert sct.tabbed(text) == text


def test_the_blank_lines_of_a_block_stay_empty_and_its_indentation_is_kept():
    text = "```python\ndef f():\n    return 1\n\n\ndef g():\n    pass\n```\n"

    assert sct.tabbed(text) == (
        '=== "Python"\n\n    ```python\n    def f():\n        return 1\n\n\n    def g():\n'
        "        pass\n    ```\n"
    )


def test_the_attributes_after_the_language_do_not_change_the_label():
    text = '```bash title="run"\nuvx botica lint\n```\n'

    assert sct.tabbed(text).startswith('=== "Bash"\n')


def test_a_page_with_no_block_is_left_as_it_is():
    text = "---\ntitle: x\n---\n\nJust text.\n"

    assert sct.tabbed(text) == text


def test_apply_rewrites_every_page_under_the_folders_it_is_given(tmp_path: Path):
    page = tmp_path / "rules" / "logging" / "a.md"
    page.parent.mkdir(parents=True)
    page.write_text("```python\nx = 1\n```\n", encoding="utf-8")
    other = tmp_path / "index.md"
    other.write_text("```python\nx = 1\n```\n", encoding="utf-8")

    sct.apply([tmp_path / "rules"])

    assert page.read_text(encoding="utf-8").startswith('=== "Python"')
    assert other.read_text(encoding="utf-8") == "```python\nx = 1\n```\n"
