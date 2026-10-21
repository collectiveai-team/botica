# The API rejects a page size above 100, though its documentation says 500 (issue #412).
PAGE_SIZE = 100


def pages(total: int) -> int:
    return -(-total // PAGE_SIZE)
