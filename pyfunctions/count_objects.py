import datetime
import json
import os
from pathlib import Path
import shutil
from typing import List, Tuple, Union

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
from scipy.ndimage import binary_dilation, binary_erosion, find_objects, label
from skimage import color
from skimage.filters import median, threshold_otsu
from skimage.morphology import disk
TYPE = "Measurement"
ITERATIONS = 7

def count_and_measure_objects(
        img: np.ndarray, mppx: float, mppy: float, outpath: Union[str, Path], downscale: int
) -> Tuple[np.ndarray, float, float]:
    config = {}
    if (p := Path("config.json")).exists():
        with open(p, "r") as fp:
            config = json.load(fp)
            print(config)

    iterations = config.get("iterations", ITERATIONS)
    outpath = Path(outpath)
    outpath, filename = outpath.parent, str(outpath.name.split(outpath.suffix)[0])
    outfile = os.path.join(outpath, "measurements.csv")
    df = pd.DataFrame()

    if os.path.isfile(outfile):
        df = pd.read_csv(outfile, index_col=[0, 1], sep=";")
    img = color.rgb2gray(img[:, :, :3])
    width, height = img.shape
    total_area = width * mppx * height * mppy * downscale**2
    thresh = threshold_otsu(img)
    binary = img > thresh
    binary = 1 - median(binary, disk(5))
    bsum = (binary > 0).sum()
    binary = binary_erosion(binary, iterations=iterations)
    lbinary = label(binary)[0]
    objs = [x for x in find_objects(lbinary) if np.prod(binary[x].shape) * mppx * mppy * downscale > 50 * 50]
    img_out = os.path.join(outpath, "object_images", filename)
    if os.path.exists(img_out):
        shutil.rmtree(img_out)
    os.makedirs(img_out)

    for i, obj in enumerate(objs):
        # If the index is not in the DataFrame, add it
        plt.imsave(os.path.join(img_out, f"{i}.jpg"), img[obj], cmap="gray")
        part = (lbinary == (i + 1)).astype('uint8')
        part = binary_dilation(part, iterations=iterations)
        entry = {
            "obj_size (mm²)": part.sum() * mppx * mppy * 1e-6,
            "measured": datetime.datetime.now(),
        }
        try:
            df.loc[(filename, i), entry.keys()] = entry
        except KeyError:
            new_entry = pd.DataFrame(
                entry,
                index=pd.MultiIndex.from_tuples([(filename, i)]),
            )

            if not len(df):
                df = new_entry
            else:
                df = pd.concat([df, new_entry])
            # If the index is already present, update the value
    df.index.names = ["slide_id", "obj_id"]
    df = df.sort_values(["slide_id", "obj_size (mm²)"])
    df.to_csv(outfile, sep=";")
    return (
        color.label2rgb(lbinary) * 255,
        (bsum / np.prod(binary.shape)) * total_area,
        len(objs),
    )


def call(
    obj: bytes,
    width: np.uint32,
    height: np.uint32,
    channels: np.uint8,
    mppx: float,
    mppy: float,
    roi: List[np.int64],
    outpath: str,
    _: str
) -> Tuple[List[float], List[str]]:
    img = np.frombuffer(obj, dtype=np.uint8).reshape(height, width, channels)
    (miny, maxy, minx, maxx) = roi
    img = img[minx:maxx, miny:maxy, :3]
    total_area = img.shape[0] * mppx * img.shape[1] * mppy * 64**2
    img, area, count = count_and_measure_objects(img, mppx, mppy, outpath, 64)
    return (
        [area * 1e-6, count, (area / total_area) * 100],
        ["Tissue (mm)²", "Nr. Objects", "Tissue/Total (%)"],
    )


def main():
    import openslide as osl
    path = "tests/data/02a7b258e875cf073e2421d67ff824cd.tiff"
    slide = osl.open_slide(path)
    level = slide.level_count - 1
    dims = slide.level_dimensions[-1]

    data = np.asarray(slide.read_region((0, 0), level, dims))
    mppx = float(slide.properties['openslide.mpp-x']) * slide.level_downsamples[level]
    mppy = float(slide.properties['openslide.mpp-y']) * slide.level_downsamples[level]
    width, height, _ = data.shape
    total_area = width * mppx * height * mppy
    _, area, count = count_and_measure_objects(data, mppx, mppy, path, 4)
    print(total_area, area, count, (area / total_area) * 100)


if __name__ == "__main__":
    main()
