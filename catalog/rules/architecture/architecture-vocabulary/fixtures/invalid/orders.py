from app.persistence.orders import OrderTable
from app.shared.ports import OrderRepository


class OrderManager(OrderRepository):
    def __init__(self, table: OrderTable) -> None:
        self.table = table
