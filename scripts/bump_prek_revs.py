"""Move the `rev` of the hooks in prek.toml and in the templates of the recipes.

A template holds parameters (`{{ param.go-preference }}`) that prek cannot parse where its schema
wants an enum, as in `language_version` of `prek-golangci-lint.toml`. So `prek update` runs on a
copy with each parameter replaced by a placeholder word, and only the lines whose `rev` changed are
written back to the template. The weekly workflow `pins.yml` runs it before `sync_pins.py`.
"""

from __future__ import annotations

import re
import subprocess
import tempfile
from pathlib import Path

import cyclopts

app = cyclopts.App(name="bump-prek-revs", help=__doc__)

PARAMETER = re.compile(r"\{\{[^}]*\}\}")
REV = re.compile(r'^rev = "[^"]*"$')
PLACEHOLDER = "system"
PREK_UPDATE = [
    "uvx",
    "prek",
    "update",
    "--cooldown-days",
    "7",
    "--exclude-tag",
    "*-rc*",
    "--exclude-tag",
    "*-beta*",
    "--exclude-tag",
    "*-alpha*",
    "--exclude-repo",
    "https://github.com/collectiveai-team/botica-pre-commit",
]


def render(template: str) -> str:
    """Return `template` with every parameter replaced by a word prek parses."""
    return PARAMETER.sub(PLACEHOLDER, template)


def carry_revs(template: str, before: str, after: str) -> str:
    """Return `template` with the `rev` lines that differ between `before` and `after`."""
    lines = template.splitlines(keepends=True)
    old = before.splitlines(keepends=True)
    new = after.splitlines(keepends=True)
    if not len(lines) == len(old) == len(new):
        raise SystemExit("prek update changed the number of lines of the file")
    for number, (was, now) in enumerate(zip(old, new, strict=True)):
        if was != now and REV.match(now.rstrip("\r\n")):
            lines[number] = now
    return "".join(lines)


def update(config: Path) -> bool:
    """Run `prek update` for `config` through a rendered copy; return whether it changed."""
    template = config.read_text(encoding="utf-8")
    if not any(REV.match(line) for line in template.splitlines()):
        return False
    rendered = render(template)
    with tempfile.TemporaryDirectory() as directory:
        copy = Path(directory) / config.name
        copy.write_text(rendered, encoding="utf-8")
        subprocess.run([*PREK_UPDATE, "--config", str(copy)], check=True)
        updated = copy.read_text(encoding="utf-8")
    new = carry_revs(template, rendered, updated)
    if new == template:
        return False
    config.write_text(new, encoding="utf-8")
    return True


@app.default
def update_command(configs: list[Path]) -> None:
    """Move the `rev` of the hooks of each config of prek."""
    for config in configs:
        if update(config):
            print(config)


if __name__ == "__main__":
    app()
