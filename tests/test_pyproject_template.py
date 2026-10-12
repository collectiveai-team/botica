"""Contract tests for the shipped pyproject template."""

from __future__ import annotations

import tomllib

from scaffolding.templates_registry import template_text


def test_ruff_leaves_the_botica_codes_in_noqa_comments_alone():
    """`# noqa: BTSTRUC001` is a botica suppression; without `external`, ruff reports RUF102."""
    config = tomllib.loads(template_text("pyproject-template.toml"))

    assert config["tool"]["ruff"]["lint"]["external"] == ["BT"]
