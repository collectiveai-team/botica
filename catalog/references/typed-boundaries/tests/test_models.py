"""The boundary functions of the topic return their structures."""

from __future__ import annotations

import pytest
from pydantic import ValidationError

from app.core.models import NewUser, Point, normalize, origin


def test_origin_returns_a_point():
    assert origin() == Point(x=0, y=0)


def test_normalize_returns_a_new_user_with_clean_fields():
    user = normalize(NewUser(email="Ana@Example.COM", display_name="  Ana "))

    assert user == NewUser(email="ana@example.com", display_name="Ana")


def test_a_new_user_validates_its_fields():
    with pytest.raises(ValidationError):
        NewUser.model_validate({"email": "ana@example.com"})
