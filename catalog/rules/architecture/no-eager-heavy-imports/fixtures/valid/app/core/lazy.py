lazy import torch
lazy from transformers import AutoModel


def embed(texts: list[str]):
    return AutoModel.from_pretrained("m")(torch.tensor(texts))
