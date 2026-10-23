"""The endpoints of users, over the port of the repository."""

from typing import Annotated

from fastapi import APIRouter, Depends, HTTPException, status

from app.api.v1.schemas.requests.users import CreateUserRequest
from app.api.v1.schemas.responses.users import UserResponse
from app.core.users import UserRepository
from app.database.services.session import get_user_repository

router = APIRouter(prefix="/users", tags=["users"])

Users = Annotated[UserRepository, Depends(get_user_repository)]


@router.post("", status_code=status.HTTP_201_CREATED)
def create_user(payload: CreateUserRequest, users: Users) -> UserResponse:
    """Register a user; an email that is taken is a 409."""
    if users.get_by_email(payload.email) is not None:
        raise HTTPException(status.HTTP_409_CONFLICT, "email taken")
    return UserResponse.of(users.add(payload.email))


@router.get("/{email}")
def read_user(email: str, users: Users) -> UserResponse:
    """Return the user with this email, or a 404."""
    user = users.get_by_email(email)
    if user is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "no such user")
    return UserResponse.of(user)
