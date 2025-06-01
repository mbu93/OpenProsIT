if True:
    import sys

    sys.path.append(".")

import json
import os
from pathlib import Path
import shutil
from typing import Dict, Iterable, List, Optional, Tuple

from dicom_parser import Header
import numpy as np
from pydicom import dcmread
import pydicom
from scipy.ndimage import zoom
from skimage.transform import resize

def center_crop(image: np.ndarray, target_shape: Iterable) -> np.ndarray:
    """
    Center crop an image to the target shape.

    Parameters:
    - image: NumPy array representing the input image.
    - target_shape: Tuple specifying the target shape of the output image, e.g., (target_height, target_width).

    Returns:
    - Cropped image as a NumPy array.
    """

    # Get the size of the input image
    height, width = image.shape[:2]

    # Get the target size
    target_height, target_width = target_shape

    # Calculate the cropping values
    crop_top = int((height - target_height) / 2)
    crop_bottom = height - crop_top
    crop_left = int((width - target_width) / 2)
    crop_right = width - crop_left

    # Perform the center crop
    cropped_image = image[crop_top:crop_bottom, crop_left:crop_right]

    return cropped_image

def rescale_dicom_image(pixel_array: np.ndarray, current_voxel_spacing: Iterable, target_voxel_spacing: Iterable) -> np.ndarray:
    """
    Rescale specified slices in a DICOM image to a specified voxel spacing.

    Parameters:
        pixel_array (np.ndarray): current data
        current_voxel_spacing (tuple): Source voxel spacing (x, y, z).
        target_voxel_spacing (tuple): Desired voxel spacing (x, y, z).

    Returns:
        np.ndarray: Rescaled DICOM dataset.
    """
    # Calculate scaling factors
    scaling_factors = current_voxel_spacing / np.array(target_voxel_spacing, dtype=float)

    # Resample specified slices
    rescaled_image = zoom(pixel_array, scaling_factors, mode='nearest')

    return rescaled_image


def flatten_dicom_dataset(ds: pydicom.FileDataset, prefix: str='') -> Dict[str, str]:
    flat_dict = {}
    for data_element in ds:
        tag = data_element.tag
        key = f"{prefix}{tag}"
        flat_dict[key] = ds[tag].value
        if data_element.VR == 'SQ':  # If it's a sequence, recursively flatten
            for _, item in enumerate(ds[tag].value):
                flat_dict.update(flatten_dicom_dataset(item))
    return flat_dict


def get_loc(x: pydicom.FileDataset) -> str:
    if hasattr(x, "SliceLocation"):
        return x.SliceLocation
    if hasattr(x, "ImagePositionPatient"):
        return x.ImagePositionPatient[2]
    flat_dict = flatten_dicom_dataset(x)
    return flat_dict["(0020, 0032)"][2]


def get_spacing(x: pydicom.FileDataset) -> str:
    if hasattr(x, "PixelSpacing"):
        return x.PixelSpacing
    flat_dict = flatten_dicom_dataset(x)
    return flat_dict["(0028, 0030)"]


def get_thickness(x: pydicom.FileDataset) -> str:
    if hasattr(x, "SliceThickness"):
        return x.SliceThickness
    flat_dict = flatten_dicom_dataset(x)
    return flat_dict["(0018, 0050)"]


def get_description(x: pydicom.FileDataset) -> str:
    if hasattr(x, "SeriesDescription"):
        return x.SeriesDescription
    flat_dict = flatten_dicom_dataset(x)
    return flat_dict["(0008, 103E)"]


def get_rows(x: pydicom.FileDataset) -> str:
    if hasattr(x, "Rows"):
        return x.Rows
    flat_dict = flatten_dicom_dataset(x)
    return flat_dict["(0028, 0011)"]


def get_columns(x: pydicom.FileDataset) -> str:
    print(type(x))
    if hasattr(x, "Columns"):
        return x.Columns
    flat_dict = flatten_dicom_dataset(x)
    return flat_dict["(0028, 0010)"]


def get_bvalue(x: pydicom.FileDataset) -> str:
    if hasattr(x, "DiffusionBValue"):
        return x.DiffusionBValue
    flat_dict = flatten_dicom_dataset(x)
    return flat_dict["(0018, 9087)"]


def get_siemens_bvalue(x: pydicom.FileDataset) -> str:
    csa = x.get(("0029", "1010")) # pyright: ignore ;works, pydicom messed up typing
    if csa is None:
        csa = x.get(("0019", "100c")) # pyright: ignore ;works, pydicom messed up typing
        return csa
    return csa["B_value"]["value"]


def crop_to_common_physical_extent(
        t2w_full: np.ndarray, adc_full: np.ndarray, hbv_full: np.ndarray, t2w_spacing: Iterable, adc_spacing: Iterable, hbv_spacing: Iterable
) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:
    """
    Crops T2W, ADC, and HBV images to the smallest physical extent based on spacing and shape.

    Args:
        t2w, adc, hbv (np.ndarray): Input images.
        *_spacing (tuple): (row_spacing, col_spacing) in mm.
        *_shape (tuple): (rows, cols).

    Returns:
        tuple: Cropped (t2w_crop, adc_crop, hbv_crop) numpy arrays.
    """

    def get_physical_size(shape, spacing):
        rows, cols = shape
        row_spacing, col_spacing = spacing
        return rows * row_spacing, cols * col_spacing  # (height_mm, width_mm)

    def get_crop_indices(image_shape, spacing, target_physical_size):
        row_spacing, col_spacing = spacing
        target_rows = int(target_physical_size[0] / row_spacing)
        target_cols = int(target_physical_size[1] / col_spacing)

        # Ensure within bounds
        target_rows = min(target_rows, image_shape[0])
        target_cols = min(target_cols, image_shape[1])

        # Center crop
        start_row = (image_shape[0] - target_rows) // 2
        start_col = (image_shape[1] - target_cols) // 2
        end_row = start_row + target_rows
        end_col = start_col + target_cols
        return slice(start_row, end_row), slice(start_col, end_col)

    t2ws = []
    adcs = []
    hbvs = []
    for t2w, adc, hbv in zip(
        t2w_full.transpose(2, 0, 1),
        adc_full.transpose(2, 0, 1)[-t2w_full.shape[-1] :],
        hbv_full.transpose(2, 0, 1)[-t2w_full.shape[-1] :],
    ):
        # Get the shapes
        t2w_shape = np.array(t2w.shape)
        adc_shape = np.array(adc.shape)
        hbv_shape = np.array(hbv.shape)

        # Compute physical sizes
        sizes_mm = [
            get_physical_size(t2w_shape, t2w_spacing),
            get_physical_size(adc_shape, adc_spacing),
            get_physical_size(hbv_shape, hbv_spacing),
        ]
        # Find smallest physical extent
        min_height = min(s[0] for s in sizes_mm)
        min_width = min(s[1] for s in sizes_mm)
        min_extent = (min_height, min_width)

        # Get crop indices for each image
        t2w_crop_idx = get_crop_indices(t2w.shape, t2w_spacing, min_extent)
        adc_crop_idx = get_crop_indices(adc.shape, adc_spacing, min_extent)
        hbv_crop_idx = get_crop_indices(hbv.shape, hbv_spacing, min_extent)

        # Crop images
        t2w_cropped = t2w[t2w_crop_idx]
        adc_cropped = adc[adc_crop_idx]
        hbv_cropped = hbv[hbv_crop_idx]
        t2ws.append(t2w_cropped)
        adcs.append(adc_cropped)
        hbvs.append(hbv_cropped)
    return (
        np.stack(t2ws).transpose(1, 2, 0),
        np.stack(adcs).transpose(1, 2, 0),
        np.stack(hbvs).transpose(1, 2, 0),
    )


def extract_layers(t2w: np.ndarray, adc: np.ndarray, hbv: np.ndarray, a: Iterable, b: Iterable, c: Iterable, theta: float) -> Tuple[np.ndarray, np.ndarray, np.ndarray]:
    result_t2w = []
    result_adc = []
    result_hbv = []
    a = [float(x) for x in a]
    b = [float(x) for x in b]
    c = [float(x) for x in c]
    for i, layer in enumerate(a):
        if all(min(abs(layer - idx) for idx in indices) < theta for indices in [a, b, c]):
            result_t2w.append(t2w[:, :, i])
    for i, layer in enumerate(b):
        if all(min(abs(layer - idx) for idx in indices) < theta for indices in [a, b, c]):
            result_adc.append(adc[:, :, i])
    for i, layer in enumerate(c):
        if all(min(abs(layer - idx) for idx in indices) < theta for indices in [a, b, c]):
            result_hbv.append(hbv[:, :, i])

    result_t2w = np.stack(result_t2w, axis=-1)
    result_adc = np.stack(result_adc, axis=-1)
    result_hbv = np.stack(result_hbv, axis=-1)

    return result_t2w, result_adc, result_hbv


class MRIExtractor:
    _t2w_name = "t2_tse_tra"
    _adc_name = [
        "ep2d_diff_b50_500_1000_tra_ADC",
        "ep2d_diff_b50_500_1000_1500_ADC",
        "ep2d_diff 0_500_1000_1500_ADC",
        "dDWI_3b B0 B100 B1500 ADC",
        "eDWI 0/500/1000 SENSE",
    ]
    _hbv_name = [
        "ep2d_diff_b50_500_1000_tra_HBV",
        "ep2d_diff_b50_500_1000_1500",
        "ep2d_diff 0_500_1000_1500",
        'DWI_3b B0 B100 B1500',
        'ep2d_diff_b50_2000_ORIG',
        'DWI_3b B0 B100 B1500',
        'DWI 0/50/500/1000',
        "EP2D_DIFF_TRA_B800_B1600_P2_160",
        "EP2D_DIFF_B50_1000_CALC1500_TRA",
    ]

    def __init__(self, path: Path):
        self.paths = list(
            set(
                [
                    f.parent
                    for f in path.rglob("*")
                    if f.is_file()
                    and f.suffix not in ["jpg", "png", "jpeg"]
                    and f.name != "DICOMDIR"
                ]
            )
        )

        self.t2w: Optional[np.ndarray] = None
        self.adc: Optional[np.ndarray] = None
        self.hbv: Optional[np.ndarray] = None
        self.idxs: Optional[Iterable] = [0, 1, 2]

    def extract(self):
        all_ds = [(dcmread(list(Path(x).rglob("*"))[0]), x) for x in sorted(self.paths)]
        if len(self.paths) < 2:
            all_ds = [(dcmread(x), x) for x in sorted(list(Path(self.paths[0]).rglob("*")))]
        t2w_positions = []
        adc_positions = []
        hbv_positions = []
        t2w_spacings = [0.5, 0.5, 3]
        adc_spacings = [0.5, 0.5, 3]
        hbv_spacings = [0.5, 0.5, 3]
        for ds in all_ds:
            descr = get_description(ds[0]).upper()
            is_t2w = (
                'T2' in descr and "COR" not in descr and "SAG" not in descr
            ) or descr in self._t2w_name
            is_adc = (
                (
                    'ADC' in descr
                    or "APPARENT DIFFUSION COEFFICIENT" in descr
                    or descr in [x.upper() for x in self._adc_name]
                )
                and not 'EADC' in descr
            ) and not descr in [x.upper() for x in self._hbv_name]
            is_dwi = (
                'BVAL' in descr
                or "TRACE" in descr
                or 'DWI' in descr
                or descr in [x.upper() for x in self._hbv_name]
            ) and not descr in [x.upper() for x in self._adc_name]

            paths = sorted(
                [str(x) for x in Path(ds[1]).glob("*")], key=lambda x: get_loc(dcmread(x))
            )
            if len(self.paths) < 2:
                paths = [ds[1]]
            if is_t2w:
                t2w_spacings = []
                imgs = []
                for p in paths:
                    dcm = dcmread(p)
                    t2w_positions.append(get_loc(dcm))
                    imgs.append(dcm.pixel_array)
                    t2w_spacings += [(*get_spacing(dcm), get_thickness(dcm))]
                imgs = np.stack(imgs)
                t2w_spacings = np.stack(t2w_spacings).mean(axis=0)
                self.t2w = imgs
            if is_adc:
                adc_spacings = []
                imgs = []
                for p in paths:
                    dcm = dcmread(p)
                    imgs.append(dcm.pixel_array)
                    adc_positions.append(get_loc(dcm))
                    adc_spacings += [(*get_spacing(dcm), get_thickness(dcm))]
                adc_spacings = np.stack(adc_spacings).mean(axis=0)
                imgs = np.stack(imgs)
                self.adc = imgs
            if is_dwi:
                hbv_spacings = []
                imgs = []

                # get the bvalue closest to 1500
                bvalues = []
                count = 0
                for x in paths:
                    try:
                        bvalues.append(int(get_bvalue(dcmread(x))))
                    except KeyError:
                        try:
                            bvalues.append(int(get_siemens_bvalue(Header(x))))
                        except Exception as e:
                            count += 1
                            print(f"{count} / {len(paths)} Error {e} for {str(x)}")
                            bvalues.append(0)
                bvalues = np.array(bvalues)
                diff = np.abs((1500 - bvalues))
                best_b = bvalues[np.argsort(diff)][0]
                for p, b in zip(paths, bvalues):
                    if b != best_b:
                        continue
                    dcm = dcmread(p)
                    imgs.append(dcm.pixel_array)
                    hbv_positions.append(get_loc(dcm))

                    hbv_spacings += [(*get_spacing(dcm), get_thickness(dcm))]
                hbv_spacings = np.stack(hbv_spacings).mean(axis=0)
                try:
                    imgs = np.stack(imgs)
                except ValueError:
                    # There are multiple images concatenated in the series
                    continue
                self.hbv = imgs  # rescale_dicom_image(imgs, hbv_spacings, (3, 0.5, 0.5))

        assert self.t2w is not None
        assert self.adc is not None
        assert self.hbv is not None
        if len(self.t2w.shape) > 3:
            self.t2w = self.t2w[0]
            t2w_positions = [t2w_positions[0]] * len(self.t2w.shape)
        if len(self.adc.shape) > 3:
            self.adc = self.adc[0]
            adc_positions = [adc_positions[0]] * len(self.adc.shape)
        if len(self.hbv.shape) > 3:
            self.hbv = self.hbv[0]
            hbv_positions = [hbv_positions[0]] * len(self.hbv.shape)
        self.t2w, self.adc, self.hbv = extract_layers(
            self.t2w.transpose(1, 2, 0),
            self.adc.transpose(1, 2, 0),
            self.hbv.transpose(1, 2, 0),
            t2w_positions,
            adc_positions,
            hbv_positions,
            5.0,
        )
        self.t2w, self.adc, self.hbv = crop_to_common_physical_extent(
            self.t2w, self.adc, self.hbv, t2w_spacings[:2], adc_spacings[:2], hbv_spacings[:2]
        )
        self.t2w = rescale_dicom_image(self.t2w, t2w_spacings, (0.5, 0.5, 3))
        self.hbv = rescale_dicom_image(self.hbv, hbv_spacings, (0.5, 0.5, 3))
        self.adc = rescale_dicom_image(self.adc, adc_spacings, (0.5, 0.5, 3))
        self.adc = rescale_dicom_image(self.adc, self.t2w.shape, self.adc.shape)
        self.hbv = rescale_dicom_image(self.hbv, self.t2w.shape, self.hbv.shape)


def norm(img: np.ndarray) -> np.ndarray:
    """Scale image between 0 and 1
    Argmuents:
    - img (np.ndarray): The image to Scale

    Returns:
    - Scaled image

    """
    return (img - img.min()) / (img.max() - img.min() + 1e-9)

class ExtractionError(Exception):
    pass

def call(
    _obj: Optional[List[np.uint8]],
    _width: Optional[np.uint32],
    _height: Optional[np.uint32],
    _channels: Optional[np.uint8],
    _mppx: Optional[float],
    _mppy: Optional[float],
    _roi: Optional[List[np.int64]],
    _outpath: Optional[str],
    inpath: str = str(Path("tests") / "MRI Test"),
):
    patient = Path(inpath)
    outpath = Path("data") /  "preprocessed" / patient.name
    if not outpath.exists():
        os.makedirs(outpath)
    else:
        return ([0], [f"patient: '{patient.name}' already exists at '{outpath}'."])
    try:
        print(f"Processing for {patient.name} with path {patient}.")
        extractor = MRIExtractor(patient)
        extractor.extract()
        t2w = extractor.t2w.transpose(2, 0, 1)
        adc = extractor.adc.transpose(2, 0, 1)
        hbv = extractor.hbv.transpose(2, 0, 1)
        if t2w is None or adc is None or hbv is None:
            raise ExtractionError(f"Error processing for {patient.name}: No  with path {patient}.")
        if t2w.shape[1] > 384:
            t2w = np.stack([center_crop(x, (384, 384)) for x in t2w])
            adc = np.stack([center_crop(x, (384, 384)) for x in adc])
            hbv = np.stack([center_crop(x, (384, 384)) for x in hbv])
        t2w = np.stack([resize(x, (224, 224), order=0) for x in t2w])
        adc = np.stack([resize(x, (224, 224), order=0) for x in adc])
        hbv = np.stack([resize(x, (224, 224), order=0) for x in hbv])
        t2w = resize(t2w, (22, 224, 224), order=0)
        adc = resize(adc, (22, 224, 224), order=0)
        hbv = resize(hbv, (22, 224, 224), order=0)
        whole = np.concatenate([
            np.stack([norm(x) for x in t2w]),
            np.stack([norm(x) for x in adc]),
            np.stack([norm(x) for x in hbv]),
            ], axis=2).transpose(1, 2, 0).astype(np.float32)
        np.save(outpath / "whole.npy", whole)

        with open(Path("data") / "stats.json", "r") as fp:
            stats = json.load(fp)

        t2w = np.stack([norm(x) for x in t2w])
        hbv = np.stack([norm(x) for x in hbv])
        adc = np.clip(adc, stats["p005"], stats["p995"])
        adc = (adc - stats["mean"]) / stats["std"]

        std = [0.229, 0.224, 0.225]
        mean = [0.485, 0.456, 0.406]
        t2w = (t2w - mean[0]) / std[0]
        adc = (adc - mean[1]) / std[1]
        hbv = (hbv - mean[2]) / std[2]
        np.save(outpath / "0000.npy", t2w)
        np.save(outpath / "0001.npy", adc)
        np.save(outpath / "0002.npy", hbv)

        whole = np.stack([t2w, adc, hbv], 3)
        whole = np.concatenate([whole[...,0], whole[...,1], whole[...,2]], axis=2).transpose(1, 2, 0)
        np.save(outpath / "whole_inp.npy", whole.astype('float32'))
        return ([0], [f"'{patient.name}' created at '{outpath}'."])
    except Exception as e:
        shutil.rmtree(outpath)
        raise e


def main():
    print(call(None, None, None, None, None, None, None, None, Path("tests") / "MRI Test"))


if __name__ == "__main__":
    main()
