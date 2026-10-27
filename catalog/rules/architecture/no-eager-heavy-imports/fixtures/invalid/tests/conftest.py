import pytest
import torch


@pytest.fixture
def batch() -> torch.Tensor:
    return torch.zeros(2, 3)
