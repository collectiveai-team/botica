"""An endpoint that reads the settings through a dependency, so a test can replace them."""

from typing import Annotated

from fastapi import APIRouter, Depends

from app.api.v1.schemas.responses.status import StatusResponse
from app.settings import Settings, get_settings

router = APIRouter()


@router.get("/status")
def read_status(settings: Annotated[Settings, Depends(get_settings)]) -> StatusResponse:
    """Say whether the service runs in debug mode."""
    return StatusResponse(debug=settings.debug)
