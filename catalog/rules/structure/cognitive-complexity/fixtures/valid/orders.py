def total(order):
    if not order.items:
        return 0
    return sum(item.price for item in order.items if item.in_stock)


def legacy_total(order):  # botica: ignore[cognitive-complexity]
    result = 0
    if order.items:
        for item in order.items:
            if item.in_stock:
                result += item.price
    return result
