def total(order):
    if not order.items:  # pragma: no cover
        return 0
    return sum(item.price for item in order.items)


def legacy():  #PRAGMA:NO COVER
    pass
