use crate::{
    error::ErrorKind,
    gui_components::Message,
    predictor::{Predictor, PredictorArgs, PreprocessingData, PreprocessingDims},
};

use iced::{advanced::subscription::EventStream, futures::stream::BoxStream};
use libvips::{
    ops::{self, BandFormat},
    VipsImage,
};
use ndarray::{s, Array, Array1, Array3, Array4, Axis};
use openslide_rs::traits::Slide;
use openslide_rs::OpenSlide;
use serde_json::Value;
use std::fs::File;
use std::hash::Hash;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::{cmp, sync::mpsc};
use tch::{CModule, Device, Tensor};

fn create_patch_grid(image_array: Array3<u8>) -> Array4<u8> {
    // Define patch size and number of patches in each dimension
    let (image_height, image_width, _) = image_array.dim();
    let patch_width = 224;
    let patch_height = 224;
    let num_patches_x = image_width / patch_width;
    let num_patches_y = image_height / patch_height;
    let num_patches_total = num_patches_x * num_patches_y;

    // Initialize 4-dimensional array to store patch grid
    let mut patch_grid =
        Array4::<u8>::zeros((num_patches_total as usize, patch_width, patch_height, 3));

    // Extract patches from the image
    for y in 0..num_patches_y {
        for x in 0..num_patches_x {
            let left = x * patch_width;
            let top = y * patch_height;
            let width = cmp::min(patch_width as usize, image_width as usize - left as usize);
            let height = cmp::min(patch_height as usize, image_height as usize - top as usize);
            let patch_view = image_array.slice(s![top..top + height, left..left + width, ..]);
            let mut patch_grid_view =
                patch_grid.index_axis_mut(Axis(0), (y * num_patches_x + x) as usize);
            patch_grid_view.assign(&patch_view);
        }
    }

    patch_grid
}

fn filter_background(array_: Array4<u8>) -> (Array4<u8>, Array1<f32>) {
    // Sum along the first dimension
    let array = array_.map(|x| *x as u32);
    let summed_array = array.sum_axis(Axis(3));

    // Create a vector to store positions where the average exceeds the threshold
    let mut background_mask: Vec<f32> = Vec::new();
    let mut filtered_arr: Array4<u8> = Array4::zeros((25, 224, 224, 3));
    //let zero_patch: Array3<u8> = Array3::zeros((224, 224, 3));
    // Iterate over the first dimension of the summed array
    for (i, slice) in summed_array.outer_iter().enumerate() {
        // Calculate the average of each (224, 224) entry
        let average: u32 = slice.sum() / (224 * 224);
        // Check if the average is higher than 3*230
        let img = array.slice(s![i, 0..224, 0..224, 0..3]).map(|x| *x as u8);
        filtered_arr.slice_mut(s![i, .., .., ..]).assign(&img);
        if (average > (3 * 230)) | (average < 10) {
            // If so, store the position
            background_mask.push(0.);
        } else {
            background_mask.push(1.);
        }
    }
    let background_mask = Array::from_vec(background_mask);
    return (filtered_arr.to_owned(), background_mask);
}

fn fetch(img: &VipsImage, posx: u32, posy: u32) -> Result<(Tensor, Tensor), ErrorKind> {
    let array: Array4<u8>; // shape (25, 224, 224, 3)

    match ops::extract_area(img, posx as i32, posy as i32, 224 * 5, 224 * 5) {
        Ok(patch) => {
            let data = patch.image_write_to_memory();
            let channels = data.len() / (224 * 5 * 224 * 5);
            let data_arr =
                Array::from_shape_vec((224 * 5, 224 * 5, channels), data).map_err(|err| {
                    ErrorKind::FetchError(
                        String::from("Couldn't create array from data!"),
                        posx,
                        posy,
                        err.to_string(),
                    )
                    .into()
                })?;
            let data_3c = data_arr.slice(s![0..224 * 5, 0..224 * 5, 0..3]);
            array = create_patch_grid(data_3c.into_owned());
        }
        Err(err) => {
            return Err(ErrorKind::FetchError(
                String::from("Couldn't fetch at x: {}, y: {}! {}"),
                posx,
                posy,
                err.to_string(),
            )
            .into())
        }
    };
    let (farray, background_mask) = filter_background(array.clone());
    let n = farray.shape()[0];
    let farray = farray.as_standard_layout().to_owned(); // farray.as_standard_layout().to_owned();
    let t = farray
        .as_slice()
        .ok_or("Couldn't get array slice!")
        .map_err(|err| {
            ErrorKind::FetchError(err.to_string(), posx, posy, err.to_string()).into()
        })?;
    let tens_u8 = Tensor::from_slice(t)
        .view((n as i64, 224, 224, 3))
        .permute([0, 3, 1, 2]);
    let tens = tens_u8.to_kind(tch::Kind::Float) / 255.;

    let back_tens = Tensor::from_slice(
        background_mask
            .as_slice()
            .ok_or("Couldn't slice mask!")
            .map_err(|err| {
                ErrorKind::FetchError(err.to_string(), posx, posy, err.to_string()).into()
            })?,
    );
    return Ok((tens, back_tens));
}

fn extend(img: &VipsImage) -> VipsImage {
    let patch_size = 224 * 5;
    let width = ((img.get_width() + patch_size - 1) / patch_size) * patch_size;
    let height = ((img.get_height() + patch_size - 1) / patch_size) * patch_size;

    //# Extend the img with ones

    let padded = ops::embed_with_opts(
        img,
        0,
        0,
        width,
        height,
        &ops::EmbedOptions {
            background: Vec::from([255., 255., 255.]),
            extend: ops::Extend::White,
        },
    )
    .unwrap_or_else(|_| {
        println!("Couldn't pad imgae");
        img.clone()
    });
    return padded;
}

/// Prepare the path for loading/saving of the prediction pendant. Simply replaces the suffix with
/// ".pred.tiff"
///
/// Example:
///
/// ```
/// # use slideslib::slide_predictor::replace_suffix_with_pred;
/// let path = replace_suffix_with_pred("foo.svs".into());
/// assert_eq!(path, String::from("foo.pred.tiff"));
/// ```
pub fn replace_suffix_with_pred(path: &str) -> String {
    // Find the position of the last occurrence of the '.' character
    if let Some(dot_index) = path.rfind('.') {
        // Create a new string with the prefix before the '.' and ".pred" appended
        let mut new_path = String::with_capacity(dot_index + 6); // 6 is the length of ".pred"
        new_path.push_str(&path[..dot_index]); // Append the prefix
        new_path.push_str(".pred.tiff"); // Append ".pred"
        new_path
    } else {
        // If the path has no '.' character, return the path unchanged
        String::from(path)
    }
}

pub struct SlidePredictor {
    pub n_tiles: usize,
    pub done: bool,
    pub image_path: PathBuf,
    out_path: String,
    backbone: CModule,
    extractor: CModule,
}

fn restore(patches: &Tensor, width: &u32, height: &u32) -> Tensor {
    // Initialize the original image
    let original_image = Tensor::zeros(
        &[*height as i64 * 5, *width as i64 * 5],
        (tch::Kind::Float, Device::Cpu),
    );

    // Reconstruct the original image
    for patch_row in 0..*height as i64 {
        for patch_col in 0..*width as i64 {
            let start_idx = ((patch_row * *width as i64 + patch_col) * 25) as usize;
            let patch = patches.narrow(0, start_idx as i64, 25);
            original_image
                .narrow(0, patch_row * 5, 5)
                .narrow(1, patch_col * 5, 5)
                .copy_(&patch.reshape(&[5, 5])); //.transpose(1, 0));
        }
    }
    original_image
}

fn map_to_rgb(tensor: &Tensor) -> Vec<u8> {
    let mut rgb_data: Vec<u8> = Vec::new();
    for row in 0..tensor.size()[1] {
        for col in 0..tensor.size()[0] {
            let value = tensor.double_value(&[col as i64, row as i64]);
            // Colorspace is BGRa
            let color = match value as i32 {
                0 => [255, 255, 255, 255],
                1 => [209, 206, 2, 255],
                2 => [26, 240, 48, 255],
                3 => [31, 95, 254, 255],
                4 => [111, 9, 179, 255],
                _ => panic!("Invalid value in tensor"),
            };
            rgb_data.extend_from_slice(&color);
        }
    }
    return rgb_data;
}

impl Predictor for SlidePredictor {
    /// Return the maximum cycle for a progress bar. Equal to the number of tiles.
    ///
    /// Example
    /// ```
    /// # use slideslib::slide_predictor::SlidePredictor;
    /// # use slideslib::error::ErrorKind;
    /// # use std::fs;
    /// # use slideslib::predictor::{Predictor, PredictorArgs};
    /// # use std::path::PathBuf;
    /// let args = PredictorArgs {width: 0, height: 0, depth: 0, path: PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff")};
    /// let predictor = SlidePredictor::new(args)?;
    /// assert_eq!(predictor.max_progress(), predictor.n_tiles);
    /// Ok::<(), ErrorKind>(())
    fn max_progress(&self) -> usize {
        return self.n_tiles;
    }

    /// Create a new slide predictor instance. Will load the ismil prediction models parts
    /// (backbone and extractor).
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::slide_predictor::SlidePredictor;
    /// # use slideslib::predictor::PredictorArgs;
    /// # use slideslib::error::ErrorKind;
    /// # use slideslib::predictor::Predictor;
    /// # use std::fs;
    /// # use std::path::PathBuf;
    /// let args = PredictorArgs {width: 0, height: 0, depth: 0, path: PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff")};
    /// let predictor = SlidePredictor::new(args.clone())?;
    /// fs::rename("models", "models_");
    /// let predictor = SlidePredictor::new(args.clone());
    /// fs::rename("models_", "models");
    /// assert!(predictor.is_err(), "Model displacement not detected!");
    /// Ok::<(), ErrorKind>(())
    /// ```
    fn new(predictor_args: PredictorArgs) -> Result<Self, ErrorKind> {
        let backbone = tch::CModule::load("models/wsi.backbone.pth")
            .map_err(|err| ErrorKind::BackboneLoadError(err.to_string()).into())?;
        let extractor = tch::CModule::load("models/wsi.extractor.pth")
            .map_err(|err| ErrorKind::ExtractorLoadError(err.to_string()).into())?;

        return Ok(Self {
            n_tiles: 0,
            done: false,
            image_path: PathBuf::from(predictor_args.path.clone()),
            out_path: replace_suffix_with_pred(
                predictor_args.path.as_os_str().to_str().unwrap_or(""),
            ),
            backbone,
            extractor,
        });
    }
    /// Preprocess the image to get the original, output and model-compatible, resized dimensions.
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::slide_predictor::{SlidePredictor};
    /// # use slideslib::error::ErrorKind;
    /// # use slideslib::predictor::PredictorArgs;
    /// # use slideslib::predictor::{Predictor, PreprocessingData};
    /// # use openslide_rs::Size;
    /// # use libvips::{VipsImage};
    /// # use std::path::PathBuf;
    /// # fn main() -> Result<(), slideslib::error::ErrorKind> {
    /// let img = VipsImage::new_from_file("tests/data/mock.tiff")
    ///            .map_err(|err| ErrorKind::VipsOpError("tests/data/mock.tiff".into(),
    ///                     err.to_string()))?;
    /// let args = PredictorArgs {width: 0, height: 0, depth: 0, path: PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff")};
    /// let mut predictor = SlidePredictor::new(args)?;
    /// let preprocessed = predictor.preprocess()?;
    /// assert!(preprocessed.is_some());
    /// let preprocessed = preprocessed.unwrap();
    /// assert_eq!(preprocessed.owidth, 5504);
    /// assert_eq!(preprocessed.oheight, 1152);
    /// assert_eq!(preprocessed.nwidth, 5600);
    /// assert_eq!(preprocessed.nheight, 2240);
    /// assert_eq!(preprocessed.outdims.w, 1376);
    /// assert_eq!(preprocessed.outdims.h, 288);
    ///
    /// Ok::<(), ErrorKind>(())
    /// # }
    /// ```
    fn preprocess(&mut self) -> Result<Option<PreprocessingData>, ErrorKind> {
        let slide = OpenSlide::new(&self.image_path)
            .map_err(|_| ErrorKind::OpenSlideMetaLoadingError(self.image_path.clone()).into())?;

        let mut default_level = 1;
        if let Ok(file) = File::open("config.json") {
            let reader = BufReader::new(file);
            let config: Value =
                serde_json::from_reader(reader).map_err(|_| ErrorKind::ConfigError())?;
            default_level = config
                .get("prediction_resolution_level")
                .and_then(Value::as_u64)
                .unwrap_or(1) as u32;
        }

        let dims = slide
            .get_level_dimensions(default_level)
            .map_err(|_| ErrorKind::OpenSlidePropertiesError(self.image_path.clone()).into())?;
        let max_dims = slide
            .get_level_dimensions(0)
            .map_err(|_| ErrorKind::OpenSlidePropertiesError(self.image_path.clone()).into())?;
        let mut outdims = openslide_rs::Size {
            w: dims.w as u32,
            h: dims.h as u32,
        };
        if let Some(ds) = slide
            .get_all_level_downsample()
            .map_err(|_| ErrorKind::OpenSlidePropertiesError(self.image_path.clone()).into())?
            .last()
        {
            let width = max_dims.w / *ds as u32;
            let height = max_dims.h / *ds as u32;
            outdims.w = width;
            outdims.h = height;
        }

        let img = ops::thumbnail_with_opts(
            self.image_path.clone().to_str().unwrap_or(""),
            dims.w as i32,
            &ops::ThumbnailOptions {
                height: dims.h as i32,
                size: ops::Size::Both,
                import_profile: "sRGB".into(),
                export_profile: "sRGB".into(),
                ..ops::ThumbnailOptions::default()
            },
        )
        .map_err(|err| {
            ErrorKind::VipsOpError(
                String::from(self.image_path.to_str().unwrap_or("")),
                err.to_string(),
            )
            .into()
        })?;
        let owidth = img.get_width();
        let oheight = img.get_height();
        let img = extend(&img);
        let nheight = img.get_height() as u32;
        let nwidth = img.get_width() as u32;

        let cols = nwidth / (224 * 5);
        let rows = nheight / (224 * 5);
        self.n_tiles = (cols * rows) as usize;
        Ok(Some(PreprocessingData {
            img,
            owidth,
            oheight,
            nwidth,
            nheight,
            outdims,
        }))
    }

    /// Run the prediction for the new image. A tx is required, as this is supposed to be executed
    /// in a separate thread, due to the possibly long duration of the prediciton procedure.
    /// Furthermore, this function optionally takes values of the preprocessing.
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::slide_predictor::{SlidePredictor};
    /// # use slideslib::error::ErrorKind;
    /// # use slideslib::predictor::PredictorArgs;
    /// # use slideslib::predictor::{Predictor, PreprocessingData};
    /// # use std::fs;
    /// # use std::path::PathBuf;
    /// # use tch::Tensor;
    /// # use libvips::{VipsImage};
    /// # use openslide_rs::Size;
    /// # fn main() -> Result<(), slideslib::error::ErrorKind> {
    /// # use std::sync::mpsc::channel;
    /// let (sender, _) = channel();
    /// let args = PredictorArgs {width: 0, height: 0, depth: 0, path: PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff")};
    /// let mut predictor = SlidePredictor::new(args)?;
    /// let img = VipsImage::new_from_file("tests/data/mock.tiff")
    ///            .map_err(|err| ErrorKind::VipsOpError("tests/data/mock.tiff".into(),
    ///                     err.to_string()))?;
    /// let data = PreprocessingData { img, owidth: 1120, oheight: 1120, nwidth: 1120,
    ///                                nheight: 1120, outdims: Size { w: 70, h: 70 }};
    /// let (raw_preds, pred_val) = predictor.run(Some(data), None, sender)?;
    ///
    /// assert_eq!(((raw_preds.mean(tch::Kind::Float) - 0.0784).abs()).double_value(&[]) < 0.1, true);
    /// assert_eq!(pred_val.mean(tch::Kind::Float).double_value(&[]), 0.);
    /// Ok::<(), ErrorKind>(())
    /// # }
    /// ```
    fn run(
        &mut self,
        preprocessed: Option<PreprocessingData>,
        preprocessing_dims: Option<PreprocessingDims>,
        tx: mpsc::Sender<Message>,
    ) -> Result<(Tensor, Tensor), ErrorKind> {
        let (img, mut owidth, mut oheight, mut nwidth, mut nheight, mut outdims): (
            VipsImage,
            i32,
            i32,
            u32,
            u32,
            openslide_rs::Size,
        ) = match preprocessed {
            None => {
                let data: PreprocessingData = self
                    .preprocess()?
                    .expect("Fatal error when collecting preprocessing data");
                (
                    data.img,
                    data.owidth,
                    data.oheight,
                    data.nwidth,
                    data.nheight,
                    data.outdims,
                )
            }
            Some(data) => (
                data.img,
                data.owidth,
                data.oheight,
                data.nwidth,
                data.nheight,
                data.outdims,
            ),
        };
        if let Some(dims) = preprocessing_dims {
            owidth = dims.owidth;
            oheight = dims.oheight;
            nwidth = dims.nwidth;
            nheight = dims.nheight;
            outdims = dims.outdims;
        }
        let mut preds: Vec<Tensor> = Vec::new();
        let mut background_mask = Tensor::from_slice::<f32>(&[]);
        let cols = nwidth / (224 * 5);
        let rows = nheight / (224 * 5);
        for row in 0..rows {
            for col in 0..cols {
                let posx = col * (224 * 5);
                let posy = row * (224 * 5);
                let (region, background_mask_) = fetch(&img, posx, posy)?;
                if background_mask
                    .size1()
                    .map_err(|err| ErrorKind::TensorPropError(err.to_string()).into())?
                    < 1
                {
                    background_mask = background_mask_;
                } else {
                    background_mask = Tensor::cat(&[background_mask, background_mask_], 0);
                }
                let feats = region.apply(&(self.extractor));
                for i in 0..25 {
                    preds.push(feats.get(i).unsqueeze(0).apply(&(self.backbone)));
                }
                tx.send(Message::UpdateCounter).unwrap_or(());
            }
        }
        let preds_ = Tensor::cat(&preds, 0);
        let preds = preds_.argmax(-1, false) * background_mask;
        let img = restore(&preds, &cols, &rows).transpose(1, 0);

        let color = map_to_rgb(&img);
        let width = img.size()[0] as i32;
        let height = img.size()[1] as i32;

        // Create the colored predictions
        let vips_image = VipsImage::new_from_memory(&color, width, height, 4, BandFormat::Uchar)
            .map_err(|err| {
                ErrorKind::VipsOpError(
                    String::from(self.image_path.to_str().unwrap_or("")),
                    err.to_string(),
                )
                .into()
            })?;
        // Resize to minimum resolution
        let resized_image =
            ops::resize(&vips_image, nheight as f64 / height as f64).map_err(|err| {
                ErrorKind::VipsOpError(
                    String::from(self.image_path.to_str().unwrap_or("")),
                    err.to_string(),
                )
                .into()
            })?;

        // Crop the expanded part
        let cropped_image =
            ops::extract_area(&resized_image, 0, 0, owidth, oheight).map_err(|err| {
                ErrorKind::VipsOpError(
                    String::from(self.image_path.to_str().unwrap_or("")),
                    err.to_string(),
                )
                .into()
            })?;

        // Create a thumbnail with exact dimensions
        let resized_image = ops::thumbnail_image_with_opts(
            &cropped_image,
            outdims.w as i32,
            &ops::ThumbnailImageOptions {
                height: outdims.h as i32,
                size: ops::Size::Force,
                import_profile: "sRGB".into(),
                export_profile: "sRGB".into(),
                ..ops::ThumbnailImageOptions::default()
            },
        )
        .map_err(|err| {
            ErrorKind::VipsOpError(String::from("Unaccesible"), err.to_string()).into()
        })?;
        let saveopts = ops::TiffsaveOptions {
            tile: true,
            tile_width: 256,
            tile_height: 256,
            pyramid: true,
            compression: ops::ForeignTiffCompression::Jpeg,
            ..ops::TiffsaveOptions::default()
        };
        ops::tiffsave_with_opts(&resized_image, self.out_path.as_str(), &saveopts).map_err(
            |err| ErrorKind::VipsOpError(String::from("Unaccesible"), err.to_string()).into(),
        )?;
        return Ok((preds_, preds));
    }
}

// Custom subscription to listen for counter updates
pub struct CounterUpdateSubscription {
    pub receiver: Arc<Mutex<Receiver<Message>>>,
}

impl iced::advanced::subscription::Recipe for CounterUpdateSubscription {
    type Output = Message;

    fn hash(&self, state: &mut iced::advanced::Hasher) {
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        use iced::futures::stream::StreamExt;
        iced::futures::stream::unfold(self.receiver, |receiver| async move {
            match receiver.lock() {
                Ok(r) => match r.recv() {
                    Ok(message) => Some((message, receiver.clone())),
                    Err(_) => None,
                },
                Err(err) => {
                    println!("Couldn't get receiver lock with error: {:?}", err);
                    None
                }
            }
        })
        .boxed()
    }
}
