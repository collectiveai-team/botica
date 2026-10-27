try:
    import cv2
except ImportError:
    cv2 = None


class Reader:
    import torch

    def read(self, path: str):
        return cv2.imread(path) if cv2 else None
