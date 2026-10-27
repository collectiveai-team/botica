import os

import torch
from transformers import AutoModel


def embed(texts: list[str]) -> list[list[float]]:
    model = AutoModel.from_pretrained(os.environ["MODEL"])
    with torch.no_grad():
        return model(texts).tolist()
