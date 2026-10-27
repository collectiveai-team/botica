import cv2

__lazy_modules__ = ["cv2", "torch"]

import torch.nn


def read(path: str):
    return cv2.imread(path)
