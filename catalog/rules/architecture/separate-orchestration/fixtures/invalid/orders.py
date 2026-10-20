def place_order(cmd: PlaceOrder) -> OrderId:
    rows = db.execute("select price from items where id in :ids", ids=cmd.item_ids)
    total = 0
    for row in rows:
        if row.price > 100 and cmd.coupon:
            total += row.price * 0.9
        else:
            total += row.price
    if total > cmd.limit:
        log.warning("limit exceeded", total=total)
        raise LimitExceeded(total)
    return db.insert("orders", total=total)
