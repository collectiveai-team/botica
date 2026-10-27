__lazy_modules__ = ["cv2", "torch", "torch.nn"]

import cv2
import torch
import torch.nn
from torch import nn


def read(path: str):
    return cv2.imread(path)
