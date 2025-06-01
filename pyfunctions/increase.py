from dataclasses import dataclass
from typing import List
import numpy as np

def call(obj: List[np.uint8], width: np.uint32, height: np.uint32, channels: np.uint8) -> List[np.uint8]:
    img = np.array(obj).reshape(height, width, channels)
    return list((img + 1).flatten())



