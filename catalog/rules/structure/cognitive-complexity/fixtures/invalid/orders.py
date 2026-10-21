def total(order):
    result = 0
    if order.items:
        for item in order.items:
            if item.in_stock:
                result += item.price
    return result


def count(order):
    return len(order.items)
