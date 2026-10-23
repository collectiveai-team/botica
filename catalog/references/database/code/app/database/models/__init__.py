"""The tables of the package. Importing it registers every table on `SQLModel.metadata`."""

from app.database.models.users import UserRow

__all__ = ["UserRow"]
