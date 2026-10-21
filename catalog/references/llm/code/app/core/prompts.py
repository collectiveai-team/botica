"""The prompts of the package, read from the Markdown files of `app/prompts/`."""

from __future__ import annotations

from functools import cache
from importlib.resources import files


@cache
def load_prompt(name: str) -> str:
    """Return the text of the prompt `app/prompts/<name>.md`, read once per process."""
    return files("app.prompts").joinpath(f"{name}.md").read_text(encoding="utf-8")
