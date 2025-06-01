from typing import List, Tuple

import numpy as np


def call(
    obj: bytes,
    width: np.uint32,
    height: np.uint32,
    channels: np.uint8,
    mppx: float,
    mppy: float,
    roi: List[np.int64],
    outpath: str,
    inpath: str
) -> Tuple[List[float], List[str]]:
    return ([np.frombuffer(obj, dtype=np.uint8).reshape((width, height, channels)).mean()], ["testfield"])
