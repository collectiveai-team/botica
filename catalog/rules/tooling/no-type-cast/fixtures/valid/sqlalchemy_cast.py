import sqlalchemy as sa
from sqlalchemy import Integer, cast, select

from app.database.tables import orders


def totals() -> sa.Select[tuple[int]]:
    return select(cast(orders.c.total, Integer), sa.cast(orders.c.tax, Integer))
