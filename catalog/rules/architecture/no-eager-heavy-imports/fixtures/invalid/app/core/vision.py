__lazy_modules__ = ["cv2", "torch"]

try:
    import cv2
except ImportError:
    cv2 = None

import torch.nn


def read(path: str):
    return cv2.imread(path)
