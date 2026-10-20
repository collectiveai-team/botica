"""Copy the pins of this repository into the templates of the recipes.

Dependabot moves the actions of `.github/workflows/`, and `prek update` moves the `rev` of the hooks
of `prek.toml`. The templates of `catalog/recipes/files/` name the same actions and the same version
of uv, and no tool watches them. This script sets, in every workflow of this repository and of the
templates, each `uses: <action>@<ref>` to the highest version that this repository's workflows use,
and the `version` of `astral-sh/setup-uv` to the `rev` of `astral-sh/uv-pre-commit` in `prek.toml`.
The weekly workflow `pins.yml` runs it after `prek update`.
"""

from __future__ import annotations

import re
from pathlib import Path

import cyclopts

app = cyclopts.App(name="sync-pins", help=__doc__)

OWN_WORKFLOWS = ".github/workflows"
TEMPLATES = "catalog/recipes/files"
USES = re.compile(r"(?P<head>uses:\s*)(?P<action>[\w.-]+/[\w./-]+)@(?P<ref>[\w./-]+)")
UV_HOOKS = re.compile(
    r'repo = "https://github\.com/astral-sh/uv-pre-commit"\n\s*rev = "(?P<rev>[^"]+)"'
)
SETUP_UV = re.compile(
    r"(?P<head>uses:\s*astral-sh/setup-uv@[\w.-]+\n(?:\s+#.*\n)*\s+with:\n(?:\s+\w[\w-]*:.*\n)*?"
    r'\s+version:\s*")(?P<version>[^"]+)(?P<tail>")'
)
VERSION = re.compile(r"^v?(\d+(?:\.\d+)*)$")

# An action and the highest ref of it, as a version.
ActionTable = dict[str, str]


def _as_version(ref: str) -> tuple[int, ...] | None:
    match = VERSION.match(ref)
    return tuple(int(part) for part in match.group(1).split(".")) if match else None


def own_pins(root: Path) -> ActionTable:
    """Return the highest versioned ref that the workflows of this repository use, per action."""
    pins: ActionTable = {}
    for workflow in sorted((root / OWN_WORKFLOWS).glob("*.yml")):
        for match in USES.finditer(workflow.read_text(encoding="utf-8")):
            ref = match["ref"]
            version = _as_version(ref)
            if version is None:
                continue
            current = pins.get(match["action"])
            if current is None or version > (_as_version(current) or ()):
                pins[match["action"]] = ref
    return pins


def uv_version(root: Path) -> str:
    """Return the version of uv that the `uv-pre-commit` hook of `prek.toml` pins."""
    match = UV_HOOKS.search((root / "prek.toml").read_text(encoding="utf-8"))
    if match is None:
        raise SystemExit("prek.toml: no `rev` of astral-sh/uv-pre-commit")
    return match["rev"].removeprefix("v")


def _workflows(root: Path) -> list[Path]:
    own = sorted((root / OWN_WORKFLOWS).glob("*.yml"))
    templates = sorted(
        path for path in (root / TEMPLATES).rglob("*.yml") if "workflows" in path.parts
    )
    return own + templates


def sync(root: Path) -> list[str]:
    """Set the pins of every workflow; return the files that changed, relative to `root`."""
    pins = own_pins(root)
    uv = uv_version(root)
    changed = []
    for path in _workflows(root):
        text = path.read_text(encoding="utf-8")
        new = USES.sub(
            lambda match: (
                f"{match['head']}{match['action']}@{pins.get(match['action'], match['ref'])}"
            ),
            text,
        )
        new = SETUP_UV.sub(lambda match: f"{match['head']}{uv}{match['tail']}", new)
        if new != text:
            path.write_text(new, encoding="utf-8")
            changed.append(path.relative_to(root).as_posix())
    return changed


@app.default
def sync_command() -> None:
    """Copy the pins of this repository into its workflows and the templates of the recipes."""
    for name in sync(Path.cwd()):
        print(name)


if __name__ == "__main__":
    app()
