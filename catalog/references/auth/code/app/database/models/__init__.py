"""The tables of the package. Importing it registers every table on `SQLModel.metadata`."""

from app.database.models.refresh_tokens import RefreshTokenRow

__all__ = ["RefreshTokenRow"]
