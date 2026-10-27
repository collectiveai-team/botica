from __future__ import annotations

from typing import TYPE_CHECKING

import numpy as np

from . import torch_config

if TYPE_CHECKING:
    import torch


def embed(texts: list[str]) -> list[list[float]]:
    import torch
    from transformers import AutoModel

    model = AutoModel.from_pretrained(torch_config.MODEL)
    with torch.no_grad():
        return np.asarray(model(texts)).tolist()


def to_tensor(values: list[float]) -> torch.Tensor:
    from torch import tensor

    return tensor(values)
