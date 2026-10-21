def _by_slug(names: list[str]) -> dict[str, int]:
    return {name: len(name) for name in names}


class _Cache:
    def entries(self) -> dict:
        return {}


def build() -> list[str]:
    def index() -> dict[str, str]:
        return {}

    return list(index())
