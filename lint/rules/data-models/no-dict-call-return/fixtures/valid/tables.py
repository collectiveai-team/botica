StatusTable = dict[str, str]


def statuses(pairs: list[tuple[str, str]]) -> StatusTable:
    return dict(pairs)
