SectionTable = dict[str, list[str]]


def section(title: str, entries: list[str]) -> SectionTable:
    return {title: entries}
