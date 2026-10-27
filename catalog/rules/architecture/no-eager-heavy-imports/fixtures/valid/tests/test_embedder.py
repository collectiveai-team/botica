import torch

from app.impl.torch_embedder import TorchEmbedder


def test_embeds_one_row_per_text():
    assert TorchEmbedder().embed(["a", "b"]).shape == torch.Size([2])
