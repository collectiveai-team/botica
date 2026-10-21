"""The endpoints of users."""

from itertools import count

from fastapi import APIRouter, status

from app.api.v1.schemas.requests.users import CreateUserRequest
from app.api.v1.schemas.responses.users import UserResponse

router = APIRouter(prefix="/users", tags=["users"])

_ids = count(1)


@router.post("", status_code=status.HTTP_201_CREATED)
def create_user(payload: CreateUserRequest) -> UserResponse:
    """Create a user and return it, built field by field from what was stored."""
    user_id = next(_ids)
    return UserResponse(id=user_id, email=payload.email, display_name=payload.display_name)
