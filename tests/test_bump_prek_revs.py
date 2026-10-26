"""The hooks of a template move to a new `rev` and the template keeps its parameters.

Asserted through the public seam of `scripts/bump_prek_revs.py`: the text of a template and the
text that `prek update` leaves in its rendered copy go in, the text of the template comes out
(CES-65). The run of prek itself needs the network and is left out.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
_SPEC = importlib.util.spec_from_file_location(
    "bump_prek_revs", ROOT / "scripts" / "bump_prek_revs.py"
)
assert _SPEC is not None
assert _SPEC.loader is not None
up = importlib.util.module_from_spec(_SPEC)
sys.modules["bump_prek_revs"] = up
_SPEC.loader.exec_module(up)

TEMPLATE = """[[repos]]
repo = "https://github.com/golangci/golangci-lint"
rev = "v2.15.0"

[[repos.hooks]]
id = "golangci-lint"
language_version = { request = "default", preference = "{{ param.go-preference }}" }
"""


def bump(text: str) -> str:
    return text.replace('rev = "v2.15.0"', 'rev = "v2.16.0"')


def test_a_parameter_becomes_a_word_that_prek_parses():
    rendered = up.render(TEMPLATE)

    assert "{{" not in rendered
    assert 'preference = "system" }' in rendered


def test_a_new_rev_goes_back_into_the_template_with_its_parameters():
    rendered = up.render(TEMPLATE)

    result = up.carry_revs(TEMPLATE, rendered, bump(rendered))

    assert result == bump(TEMPLATE)
    assert "{{ param.go-preference }}" in result


def test_a_template_that_prek_leaves_alone_stays_as_it_is():
    rendered = up.render(TEMPLATE)

    assert up.carry_revs(TEMPLATE, rendered, rendered) == TEMPLATE


def test_a_line_other_than_a_rev_is_not_carried_back():
    rendered = up.render(TEMPLATE)
    edited = rendered.replace('id = "golangci-lint"', 'id = "other"')

    assert up.carry_revs(TEMPLATE, rendered, edited) == TEMPLATE


def test_a_change_in_the_number_of_lines_stops_the_run():
    rendered = up.render(TEMPLATE)

    with pytest.raises(SystemExit, match="number of lines"):
        up.carry_revs(TEMPLATE, rendered, rendered + "\n")


def test_every_template_of_this_repository_renders_to_a_parameter_free_text():
    templates = [ROOT / "prek.toml", *sorted((ROOT / "catalog/recipes").rglob("prek-*.toml"))]

    for template in templates:
        assert "{{" not in up.render(template.read_text(encoding="utf-8")), template
