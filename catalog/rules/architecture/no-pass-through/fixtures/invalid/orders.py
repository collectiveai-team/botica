from app.core.orders import Order, OrderId, Orders


class OrderService:
    def __init__(self, orders: Orders) -> None:
        self._orders = orders

    def get(self, order_id: OrderId) -> Order:
        return self._orders.get(order_id)

    def save(self, order: Order) -> None:
        self._orders.save(order)
