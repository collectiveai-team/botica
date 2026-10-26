from app.core.orders import Order, OrderId, OrderNotFound, Orders


class OrderLookup:
    def __init__(self, orders: Orders) -> None:
        self._orders = orders

    def get(self, order_id: OrderId) -> Order:
        order = self._orders.find(order_id)
        if order is None or order.deleted:
            raise OrderNotFound(order_id)
        return order
