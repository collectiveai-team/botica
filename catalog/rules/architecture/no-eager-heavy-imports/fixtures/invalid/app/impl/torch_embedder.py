import torch


class TorchEmbedder:
    def embed(self, texts: list[str]) -> torch.Tensor:
        return torch.tensor([len(text) for text in texts])
