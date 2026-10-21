def place_order(cmd: PlaceOrder) -> OrderId:
    order = build_order(cmd)
    validate(order)
    return orders.save(order)


def build_order(cmd: PlaceOrder) -> Order:
    return Order(items=prices.of(cmd.item_ids), coupon=cmd.coupon)


def validate(order: Order) -> None:
    if order.total > order.limit:
        raise LimitExceeded(order.total)
