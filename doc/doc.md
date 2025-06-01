

<h1><b>Open</b> <b>Pro</b>tate Cancer <b>I</b>nspection <b>T</b>oolkit </h1>

![LOGO](doc/openprosit.png)

This is the V0.1.0 version of the <b>Open</b> <b>Pro</b>tate Cancer <b>I</b>nspection <b>T</b>oolkit (OpenProsIT), which aims to provide a lightweight and open-source AI-analysis software for prostate cancer (PCa). OpenProsIT is written as part of my PhD works and allows you to perform some handy things like:
- Inspecting whole slide image (WSI) and magnetic resonance imaging (MRI) data in a ressource-efficient fashion. 
- Performing an AI-based grading using a model pretrained on 100.000+ slide images.
- Performing an AI-based MRI-lesion-segmentation using a model pretrained on 2000+ cases.
- Running and visualising torchscript models on WSI and MRI data.
- Incorporating Python scripts to analyse tissue / MRI data, including the capabilty to return measurement values or overlays (examples are provided).
Further information on OpenProsIT can be found in the following papers:
- <a href="https://link.springer.com/chapter/10.1007/978-3-031-54605-1_23">Self-supervised Learning in Histopathology: New Perspectives for Prostate Cancer Grading.</a>
- <a href="https://link.springer.com/chapter/10.1007/978-3-031-78398-2_24">SWJEPA: Improving Prostate Cancer Lesion Detection with Shear Wave Elastography and Joint Embedding Predictive Architectures.</a>
- <a href="https://www.scitepress.org/Documents/2024/126819">An Open-Source Approach for Digital Prostate Cancer Histopathology: Bringing AI into Practice.</a>
- 
Note that this is a early-stage research tool and thus has its perks! Contributors to the software are, however, very welcome :)  

## System Requirements
To run the tool you'll need:

- A 64bit Linux (tested on Arch Linux) or Windows (tested on Windows 10) system.
- A running Python interpreter (see [installation](##Installation)).
- The required PIP packages: scikit-image, pandas, numpy, scipy, matplotlib & numpy (see [installation](##Installation)). 
- on linux systems:
    - A working installation of openslide (4.0.0). (openslide [archlinux] openslide-tools [ubuntu])
    - A working installation of clang [arch linux and ubuntu]
    - A working installation of libvips [arch linux, ubuntu] (8.15.x; NOTE: older version *WILL NOT* work), libvips-dev [ubuntu] and libvips-tools [ubuntu].
    - header packages etc. (libopenslide-dev libopenslide0 libgtk-3-dev libvips-dev [ubuntu]
    
        for package in openslide-tools ; do
For linux users, the CI image of the repo may also be used (mbu93/openprosit-ubuntu). Note that only  ubuntu 24.04 and newer ship the right libvips version.

## Installation

### From Source
To install from source clone this repository and execute ```build-linux```or ```build-windows-native```according to your distribution. After that run ```python install_packages.py```.
In the last step you need to download the models to the ```models``` folder from these links:
- https://drive.google.com/file/d/1FNvPWubiwq-u0C4cGX45DuR2vd22rBb-/view?usp=drive_link
- https://drive.google.com/file/d/1RQrE9GuhUenSgtxP8Qih4v7FZN0Cs30_/view?usp=drive_link
- https://drive.google.com/file/d/1cJn866V5AyY7qrJSS-y1V9vIDPjiN1FO/view?usp=drive_link

### Prebuilt
Installation is relatively straightforward. First, download the required zip from the release page. Afterwards, unzip the file. You'll find a folder structure like this:

![Installation files.](doc/files.png)

You can move the folder to any location you want, but these files need to be in one folder! This is important for the software to, e.g., detect the required Python scripts. 

#### Windows
To install and run the software, three steps need to be performed:

1. Install Python3.11 or any newer Version (Tested: Windows - Python3.11, Linux: Python 3.13). Make sure to check the "add Python to path" box in the installer (it should be selected by default).
2. Run the "install_packages.py" script by simply double-clicking it. A console window will pop up, and the installation will take a few minutes. Please don't close the window; sometimes it just looks like nothing is happening anymore. The window should disappear as soon as the installation is complete.
3. Run the "rusty_slides.exe" file (or binary in case of linux). 

The remaining files are used by the software internally. For the sake of completeness here is a description of their content:

- deps: includes all the wheels of Python packages that will be installed. This ensures compatibility and reduces the installation effort (no download / package build is required).
- libopenslide-1.dll: is the binary to load the whole slide images. 
- pyfunctions: contains the script for object counting. In this folder custom script can also be added and loaded inside the software. 

#### Linux
For Linux, simply download the release zip and install libvips as well as openslide and Pytorch. Alternatively, you can build from source using make and cargo (see above).

## Usage Instructions
Currently counting of tissue objects and measuring their area is supported, as well as AI grading of WSIs and lesion detection in MRIs. See below for details  on how to analyse your data. 

### Landing Page and Menu
After starting the executable, you'll see a screen like this:
<table>
<tr>
<td align="center"><img src="doc/start.png" width="600"/><br/><sub>Landing page without menu.</sub></td>
<td align="center"><img src="doc/menu.png" width="600"/><br/><sub>Landing page with menu</sub></td>
</tr>
</table>

It contains two buttons: "Menu" and "Analyse" to load slides, folders of slides and processing scripts and to execute the latter ones. Additionally, there are two information windows which currently show placeholders. This is where files will be selectable and processing info will be displayed. 

Clicking on the "Menu" Button will call a 3-choice dialog as shown below. As may be self-explanatory, here you can either:

- choose a file to evaluate (WSI or DICOM),
- choose a folder with multiple WSI files / a single case DICOM folder to evaluate or,
- set the processing script.

After setting a file you can view, scroll and pan as displayed in the next figure. Scrolling works using the arrow keys ("up" for zooming in and "down" for zooming out), while dragging can be done using left-click and moving with the mouse. For MRI images, z-axis scrolling is possible using the up and down arrows. Zooming in panning is not supported in this case.

<table>
<tr>
<td align="center"><img src="doc/single_view.png" width="600"/><br/><sub>Loaded SVS list.</sub></td>
<td align="center"><img src="doc/load.png" width="600"/><br/><sub>Zoomed File.</sub></td>
</tr>
</table>

To select a different file, just click on the name in the upper info window. 

### Script-based Analysis
For processing the images, Python scripts are used alongside a native torch model integration. A default script is placed in the pyfunctions folder of the project, which will process the viewport (the __visible__ area of the image), count the total amount of tissue and calculate the percentage for each detecting piece of tissue. 

![Info.](doc/info.png)
<div align="center">Extracted information is displayed after processing.</div>

This script is triggered when the "Analyse" Button is clicked. Note that a progress bar will appear as in the lower part. Calculation may still take some time (up to a few minutes). When the script is finished an overview  of informations is displayed as below. 
The information consists of the overall amount of tissue/object glass, the tissue are in mm² and the number of detected objects. Note that the latter one may seem larger than expected, as very small parts are already counted. In addition to the overview, a csv file "measurement.csv" is created in the folder where the data is located, as well as as folder "object_images". The overall file structure looks like: 

![File tree created by the count script.](doc/tree.png)

The CSV file contains the amount, ID, slide name, processing date and size of each tissue part detected in a whole slide image. Additionally, in the "object_images" folder, the referred parts of the image (as indicated by the ID in the CSV) are plotted. You can also specify an area that you want to anayse by hitting the crop button, which will let you define a ROI:

![Cropping](doc/crop.png)

### AI-based grading. 
For WSI images ("svs", "tiff"), using the "Classify Image" button will trigger the AI-based grading. You can check the progress with the progress bar. The default level for analysis is the second highest magnification. This can be adjusted by editing the "config.json" file. Note that the highst magnification=0. After analysis, an overlay will be displayed like below 

![WSI Classification](doc/classify_wsi.png)

The colour scheme refers to:
- white (bright) - benign
- green - GG3
- light red / orange - GG4
- dark red / pinkg - GG5

The overlay can be toggled using the "AI map on" / "AI map off". Note that, right after running the analysis toggling the button may be required. The predictions are furthermore stored for later.


### AI-based Staging. 
Similar to AI-based grading, using a DICOM MRI file will allow for lesion detection. Just hit the "Classify Image" button once again, and lesions will be marked in red as below. Scrolling is possible using the up and down arrows.

![MRI Classification](doc/classify_mri.png)

The colour scheme refers to:
- white (bright) - benign
- green - GG3
- light red / orange - GG4
- dark red / pinkg - GG5

The overlay can be toggled using the "AI map on" / "AI map off". Note that, right after running the analysis toggling the button may be required. The predictions are furthermore stored for later.

## Data Format
WSI images can simply be used as is. Currently "SVS" and "TIFF" are supported (and tested). For MRI, please provide data as follows:

```
| root folder  
  | - mpMRI
    | - t2w folder
    | - adc folder
    | - dwi folder
```

Keeping this folder schweme (especially the 'mpMRI') is crucial for the software to detect the right data structure.

## Adding Custom Models / Scripts

### Python-based
One key element of this software is the so-called Python-bridge, which allows for easy integration of custom scripts. Hence, this tool can be used for various purposes beyond the scope of PCa grading/staging. Individual scripts can be selected from within the software. Each script requires a method ```call``` with the following signature.
```
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
```
The provided arguments will be used by the actual Rust-based GUI and represent the following:
- obj: the image data of the current viewport 
- width: the vieport width
- height: the vieport width
- channels: image channels (RGB -> 3, RGBA -> 4, ...)
- mppx: pixel resolution in um/px
- mppy: pixel resolution in um/px
- roi: (miny, maxy, minx, maxx) values if crop was used
- outpath: if output is stored, it's stored here
- inpath: if something should be loaded in the script, then from this path

Furthermore an script-type (as global constant) must be specified that controls the return behaviour and may be:
- ```TYPE = "Measurement"```
- ```TYPE = "Overlay"```

"Measurement" is used to, e.g. [count tissue amount](pyfunctions/count_objects.py). It returns a tuple of listed measurement values and listed names like ([1, 2, 3], ["a", "b", "c"]). An "Overlay" on the other hand [calculates a mask to be displayed](pyfunctions/overlay_mock.py), and can, e.g., be used to directly incorporate a Python-based torch model. Overlays return a tuple of a flat list of pixel values and an emptystring like ([1, 2, 3, ...], [""])m whereas the shape before flattening must be the same as the one of the input image.

### Torch-based Staging or Grading
For incorporating PyTorch models, besides using the Python-bridge, the pre-built models could be patched, which is a little hacky but totally possible with low effort. For this purpose, first, [export your model as torchscript module](https://docs.pytorch.org/docs/stable/generated/torch.jit.save.html). For WSI data, the model needs to accept stacked (3, 224, 224) patches such that the input shape is (N, 3, 224, 224). Correspondingly, the output requires the shape (N, C), whereas C is 5 or lower. In the WSI case, store your model as "wsi.backbone.pth" in the models folder and export a model that consists of an nn.Identity() as "wsi.extractor.pth". For the MRI model, the input requires the shape (N, 3, 224, 224) and the output provides a segmentation of shape [2, 2, 224, 224], wheres as the format is as follows: ((lesion|organ),(background|class), W, H).


## Compatibility Warning
It certainly has come to your attention already that (particulary medical) AI models are heavily domain dependent, which means that they may not work with hospital data different from the one they were trained with. This accounts for the models of this repo as well. While the WSI (grading) model, alongside some internal dataset, was trained using the TCGA and PANDA public datasets and should therefore be sufficiently generalistic, the MRI model definitely delivers scanner-dependet results. The latter is mainly driven by the fact that ADC, in contrast to T2 and DWI, is provided in scanner-specific, non-physical, values. The normalisation procedure is performed using the values in ```data/stats.json```. To make the algorithm more reliable, please provide own ADC value statistics (in this file) or even fine-tune the models locally!

## Acknowledgement
This project was created with the help of various academic institutes, namely:
- [University Leipzig](https://www.uni-leipzig.de/)
- [Institute for Applied Informatics Leipzig](https://infai.org/)
- [University Hospital Bonn](https://www.ukbonn.de/)
- [University Hospital Wrocław](https://www.umw.edu.pl/en)
- [ScaDS.AI Leipzig](https://scads.ai/)
 
Thanks to all the contributors, especially to Glen, Marit, Lennart and Adam! Furthermore, don't miss the chance to check the awesome open-source projects and datasets included in this work. To name only a few important ones (you find more in the papers and code):

- [iced Rust](https://book.iced.rs/)
- [ISMIL](https://github.com/YangZyyyy/Intensive-sampling-MIL)
- [CTransPath](https://github.com/Xiyue-Wang/TransPath)
- [IJEPA](https://github.com/facebookresearch/ijepa)
- [TCGA dataset](https://www.cancer.gov/ccg/research/genome-sequencing/tcga)
- [PANDA dataset](https://panda.grand-challenge.org/data/)
- [PICAI dataset](https://pi-cai.grand-challenge.org/)
- [Prostate158 dataset](https://github.com/kbressem/prostate158)

# Citation
If you like and/or use the project, please cite one of our related works.

For the tool as such:
```
@conference{iceis24,
author={Markus Bauer and Lennart Schneider and Marit Bernhardt and Christoph Augenstein and Glen Kristiansen and Bogdan Franczyk},
title={An Open-Source Approach for Digital Prostate Cancer Histopathology: Bringing AI into Practice},
booktitle={Proceedings of the 26th International Conference on Enterprise Information Systems - Volume 1: ICEIS},
year={2024},
pages={729-738},
publisher={SciTePress},
organization={INSTICC},
doi={10.5220/0012681900003690},
isbn={978-989-758-692-7},
}
```

For the WSI model:
```
@conference{iceis24,
    author={Markus Bauer and Lennart Schneider and Marit Bernhardt and Christoph Augenstein and Glen Kristiansen and Bogdan Franczyk},
    title={An Open-Source Approach for Digital Prostate Cancer Histopathology: Bringing AI into Practice},
    booktitle={Proceedings of the 26th International Conference on Enterprise Information Systems - Volume 1: ICEIS},
    year={2024},
    pages={729-738},
    publisher={SciTePress},
    organization={INSTICC},
    doi={10.5220/0012681900003690},
    isbn={978-989-758-692-7},
}
```

```
@inbook{Bauer2024,
  title = {Self-supervised Learning in Histopathology: New Perspectives for Prostate Cancer Grading},
  ISBN = {9783031546051},
  ISSN = {1611-3349},
  url = {http://dx.doi.org/10.1007/978-3-031-54605-1_23},
  DOI = {10.1007/978-3-031-54605-1_23},
  booktitle = {Pattern Recognition},
  publisher = {Springer Nature Switzerland},
  author = {Bauer,  Markus and Augenstein,  Christoph},
  year = {2024},
  pages = {348–360}
}
```

For the lesion-segmentation model:
```
@inbook{SWJEPA2024,
  title = {SWJEPA: Improving Prostate Cancer Lesion Detection with Shear Wave Elastography and Joint Embedding Predictive Architectures},
  ISBN = {9783031783982},
  ISSN = {1611-3349},
  url = {http://dx.doi.org/10.1007/978-3-031-78398-2_24},
  DOI = {10.1007/978-3-031-78398-2_24},
  booktitle = {Pattern Recognition},
  publisher = {Springer Nature Switzerland},
  author = {Bauer,  Markus and Gurwin,  Adam and Augenstein,  Christoph and Franczyk,  Bogdan and Małkiewicz,  Bartosz},
  year = {2024},
  month = dec,
  pages = {359–375}
}
```

## Issues
As this is an ongoing project, of course there is tons of bugs and limitations. Currently the following things are known and under construction:
- The viewer has a caching mechanism that allows for scrolling seamlessly. However, when the cache is updated, a slight delay is recognisable.
- Behaviour on the edges of the images can sometimes by unpredictable. Please be patient in such case.
- If the CSV is corrupted, the tool won't be capable to measure anything. Please just remove the measurement.csv from the path in that case. 
