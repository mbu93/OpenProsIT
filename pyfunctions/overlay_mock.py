from typing import List, Tuple

import numpy as np
TYPE = "Overlay"

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
    return ([float(x) for x in obj], [""])
