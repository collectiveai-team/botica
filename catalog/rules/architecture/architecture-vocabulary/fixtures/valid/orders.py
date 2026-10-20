from app.core.ports import OrderRepository
from app.database.orders import OrderTable


class SqlOrderRepository(OrderRepository):
    def __init__(self, table: OrderTable) -> None:
        self.table = table
