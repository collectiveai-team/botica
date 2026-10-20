# --- constants ---
# set the page size to 100
PAGE_SIZE = 100


def pages(total: int) -> int:
    # divide the total by the page size, rounding up
    return -(-total // PAGE_SIZE)
    # old version: return total // PAGE_SIZE + 1
