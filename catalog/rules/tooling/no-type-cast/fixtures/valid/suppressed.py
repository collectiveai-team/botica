from typing import cast

from app.plugins import Plugin, registry


def plugin(name: str) -> Plugin:
    return cast(Plugin, registry[name])  # botica: ignore[no-type-cast] -- stubs of the registry
