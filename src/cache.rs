use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::vec::Vec;

// Libvips
use libvips::{ops, VipsImage};

// Openslide
use openslide_rs::properties::openslide::{
    OPENSLIDE_PROPERTY_NAME_MPP_X, OPENSLIDE_PROPERTY_NAME_MPP_Y,
};
use openslide_rs::traits::Slide;
use openslide_rs::Address;
use openslide_rs::Region;
use openslide_rs::{OpenSlide, Size as OpenslideSize};

// Numpy Arrays
use npyz;

// Local modules
use crate::error::*;
use crate::slide_predictor::replace_suffix_with_pred;
use crate::tracking::Borders;
use crate::util::log_or_load_thread_err;
use crate::ImageType;
use crate::{ZoomableImageViewer, CACHE_MAX};

pub struct Border {
    pub cache: Borders,
    pub edge: Borders,
}

#[derive(Debug, Clone)]
pub struct PreloadRegionArgs {
    pub cache_scale_factor_x: f32,
    pub cache_scale_factor_y: f32,
    pub level: u32,
    pub max_extents: OpenslideSize,
    pub cache_size: OpenslideSize,
    pub offsetx: f32,
    pub offsety: f32,
    pub image_path: Vec<PathBuf>,
    pub current_image: usize,
    pub levels: Vec<f64>,
}

impl From<&mut ZoomableImageViewer> for PreloadRegionArgs {
    fn from(data: &mut ZoomableImageViewer) -> Self {
        PreloadRegionArgs {
            cache_scale_factor_x: data.cache_scale_factor_x,
            cache_scale_factor_y: data.cache_scale_factor_y,
            level: data.level,
            max_extents: data.max_extents,
            cache_size: data.plot_data.view.cache_size,
            offsetx: data.offsetx,
            offsety: data.offsety,
            image_path: data.image_path.clone(),
            current_image: data.current_image,
            levels: data.levels.clone(),
        }
    }
}

/// Function to get the closest level of precalculated zoom levels from a WSI image. Returns the
/// index of the current level as well as the value.
///
/// Examples:
/// ```
/// # use std::vec::Vec;
/// # use std::io;
/// # use slideslib::cache::find_next_greater_value;
/// # fn main() -> Result<(), &'static str> {
/// let values = Vec::from([0., 2., 8., 16.]);
/// // Next greatest level is 2
/// let (idx, level) = find_next_greater_value(values.clone(), 2).ok_or("Wrong value!")?;
/// assert_eq!(idx, 1);
/// assert_eq!(level, 2);
/// // Now it's 8
/// let (idx, level) = find_next_greater_value(values.clone(), 5).ok_or("Wrong value!")?;
/// assert_eq!(idx, 2);
/// assert_eq!(level, 8);
/// // Still 8
/// let (idx, level) = find_next_greater_value(values.clone(), 8).ok_or("Wrong value!")?;
/// assert_eq!(idx, 2);
/// assert_eq!(level, 8);
/// # Ok(())}
/// ```
///
pub fn find_next_greater_value(slice: Vec<f64>, target: u32) -> Option<(u32, u32)> {
    for (i, &value) in slice.iter().enumerate() {
        if value >= target as f64 {
            return Some((i as u32, value as u32));
        }
    }
    None
}

/// Get a region from an openslide-readable image. Requires the following arguments:
/// - PreloadRegionArgs {
///     cache_scale_factor_x - the ration of x cache vs. viewport_size
///     cache_scale_factor_y - the ration of y cache vs. viewport_size
///     level - the current downsample level
///     max_extents - maximum extents of full resolution at current magnification
///     cache_size - cache array size
///     offsetx - x position
///     offsety - y position
///     image_path - slides to load
///     current_image - slide index to load
///     level - all available downsample levels
/// }
/// - load_pred (bool): specify whether to load a preprocessed prediction with ending 'pred.tiff'
/// and the same identifier as the WSI
/// - impath (String): the image path of the prediction file (pred.tiff)
///
/// Example:
/// ```
/// # use slideslib::cache::{PreloadRegionArgs, get_region};
/// # use slideslib::error::ErrorKind;
/// # use std::{vec::Vec, path::PathBuf, io};
/// # use openslide_rs::Size as OpenslideSize;
/// let mut args = PreloadRegionArgs {
///     cache_scale_factor_x: 2.,
///     cache_scale_factor_y: 2.,
///     level: 16,
///     max_extents: OpenslideSize { w: 22016, h: 4608 },
///     cache_size: OpenslideSize { w: 2752, h: 576 },
///     offsetx: 11008.,
///     offsety: 2304.,
///     image_path: Vec::from([PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.tiff")]),
///     current_image: 0,
///     levels: Vec::from([1., 4., 16.]),
/// };
/// let region = get_region(
///     args,
///     false,
///     String::from(PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.pred.tiff").to_str().unwrap_or("")),
/// )?;
/// let sum: u32 = region.as_slice().iter().map(|x| *x as u32).sum();
/// assert_eq!(sum, 381986274);
/// # Ok::<(), ErrorKind>(())
/// ```
///
/// Note that this code will return all-zero arrays if invalid positions are provided.
///
/// ```
/// # use slideslib::cache::{PreloadRegionArgs, get_region};
/// # use slideslib::error::ErrorKind;
/// # use std::{vec::Vec, path::PathBuf, io};
/// # use openslide_rs::Size as OpenslideSize;
/// # let args = PreloadRegionArgs {
/// #     cache_scale_factor_x: 2.,
/// #     cache_scale_factor_y: 2.,
/// #     level: 16,
/// #     max_extents: OpenslideSize { w: 22016, h: 4608 },
/// #     cache_size: OpenslideSize { w: 2752, h: 576 },
/// #     offsetx: 200000.,
/// #     offsety: 0.,
/// #     image_path: Vec::from([PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.tiff")]),
/// #     current_image: 0,
/// #     levels: Vec::from([1., 4., 16.]),
/// # };
/// // with offsetx = 200000
/// let region = get_region(
///     args,
///     false,
///     String::from(PathBuf::from("tests").join("data").join("mock.pred.tiff").to_str().unwrap_or("")),
/// )?;
/// let sum: u32 = region.as_slice().iter().map(|x| *x as u32).sum();
/// assert_eq!(sum, 0);
/// # Ok::<(), ErrorKind>(())
/// ```
///
/// This crate offers convenience functions to circumvent that by clipping global
/// [Self::tracking::cli/// p_global_coords] and cache coordinates [Self::tracking::clip_cache_coords].
/// It's also possible to read the precalculated prediction:
///
/// ```
/// # use slideslib::cache::{PreloadRegionArgs, get_region};
/// # use slideslib::error::ErrorKind;
/// # use std::{vec::Vec, path::PathBuf, io};
/// # use openslide_rs::Size as OpenslideSize;
/// # let args = PreloadRegionArgs {
/// #     cache_scale_factor_x: 2.,
/// #     cache_scale_factor_y: 2.,
/// #     level: 16,
/// #     max_extents: OpenslideSize { w: 22016, h: 4608 },
/// #     cache_size: OpenslideSize { w: 2752, h: 576 },
/// #     offsetx: 11008.,
/// #     offsety: 2304.,
/// #     image_path: Vec::from([PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.tiff")]),
/// #     current_image: 0,
/// #     levels: Vec::from([1., 4., 16.]),
/// # };
/// let region = get_region(
///     args,
///     true,
///     String::from(PathBuf::from("tests").join("data").join("mock.pred.tiff").to_str().unwrap_or("")),
/// )?;
/// let sum: u32 = region.as_slice().iter().map(|x| *x as u32).sum();
/// assert_eq!(sum, 1616855040);
/// # Ok::<(), ErrorKind>(())
/// ```
/// In that case, filtering invalid positions is mandatory as otherwise errors will occur rather
/// than invalid data.
pub fn get_region(
    data: PreloadRegionArgs,
    load_pred: bool,
    pred_path: String,
) -> Result<Vec<u8>, ErrorKind> {
    let filename = data.image_path[data.current_image].to_str().unwrap_or("");
    let p = Path::new(filename);
    let levels = data.levels;
    let last_level = levels.last().copied().unwrap_or(1.) as f32;
    let (level_idx, level) = find_next_greater_value(levels, data.level).unwrap_or((3, data.level));
    let level = level as f32;
    let mut cache_size_w = data.cache_size.w;
    let mut cache_size_h = data.cache_size.h;
    let posx = data.offsetx - cache_size_w as f32 / 2. * level;
    let posy = data.offsety - cache_size_h as f32 / 2. * level;
    let mut w = cache_size_w;
    let mut h = cache_size_h;
    if (posy < 0.) & (level != last_level) {
        h = (h as f32 - posy.abs() / level).abs() as u32
    }
    if (posx < 0.) & (level != last_level) {
        w = (w as f32 - posx.abs() / level).abs() as u32
    }
    let thumb: VipsImage;
    let ethumb: VipsImage;
    let region: Vec<u8>;
    if !load_pred {
        let slide = OpenSlide::new(p)
            .map_err(|_| ErrorKind::OpenSlideImageLoadingError(PathBuf::from(p)).into())?;
        region = slide
            .read_region(&Region {
                size: OpenslideSize {
                    w: w.max(1),
                    h: h.max(1),
                },
                level: level_idx,
                address: Address {
                    x: posx as u32,
                    y: posy as u32,
                },
            })
            .map_err(|err| {
                ErrorKind::VipsOpError(String::from(p.to_str().unwrap_or("")), err.to_string())
                    .into()
            })?;
        thumb = VipsImage::new_from_memory(
            region.as_ref(),
            w.max(1) as i32,
            h.max(1) as i32,
            4,
            ops::BandFormat::Uchar,
        )
        .map_err(|err| {
            ErrorKind::VipsOpError(
                data.image_path
                    .get(0)
                    .unwrap_or(&PathBuf::from("Missing"))
                    .clone()
                    .to_str()
                    .unwrap_or("Missing")
                    .into(),
                err.to_string(),
            )
            .into()
        })?;
        ethumb = ops::embed(
            &thumb,
            (cache_size_w - w.max(1)) as i32,
            (cache_size_h - h.max(1)) as i32,
            cache_size_w as i32,
            cache_size_h as i32,
        )
        .map_err(|err| {
            ErrorKind::VipsOpError(
                data.image_path
                    .get(0)
                    .unwrap_or(&PathBuf::from("Missing"))
                    .clone()
                    .to_str()
                    .unwrap_or("Missing")
                    .into(),
                err.to_string(),
            )
            .into()
        })?;
    } else {
        let p = PathBuf::from(pred_path.as_str());
        let slide = OpenSlide::new(p.clone().as_path())
            .map_err(|_| ErrorKind::OpenSlideImageLoadingError(PathBuf::from(p.clone())).into())?;
        w = (w * level as u32 / last_level as u32).max(1);
        h = (h * level as u32 / last_level as u32).max(1);
        cache_size_w = cache_size_w * level as u32 / last_level as u32;
        cache_size_h = cache_size_h * level as u32 / last_level as u32;
        region = slide
            .read_region(&Region {
                size: OpenslideSize { w, h },
                level: 0,
                address: Address {
                    x: (posx / last_level) as u32,
                    y: (posy / last_level) as u32,
                },
            })
            .map_err(|err| {
                ErrorKind::VipsOpError(String::from(p.to_str().unwrap_or("")), err.to_string())
                    .into()
            })?;
        thumb = VipsImage::new_from_memory(
            region.as_ref(),
            w as i32,
            h as i32,
            4,
            ops::BandFormat::Uchar,
        )
        .map_err(|err| {
            ErrorKind::VipsOpError(
                data.image_path
                    .get(0)
                    .unwrap_or(&PathBuf::from("Missing"))
                    .clone()
                    .to_str()
                    .unwrap_or("Missing")
                    .into(),
                err.to_string(),
            )
            .into()
        })?;
        ethumb = ops::embed(
            &thumb,
            (cache_size_w - w.max(1)) as i32,
            (cache_size_h - h.max(1)) as i32,
            cache_size_w as i32,
            cache_size_h as i32,
        )
        .map_err(|err| {
            ErrorKind::VipsOpError(
                data.image_path
                    .get(0)
                    .unwrap_or(&PathBuf::from("Missing"))
                    .clone()
                    .to_str()
                    .unwrap_or("Missing")
                    .into(),
                err.to_string(),
            )
            .into()
        })?;
    };
    let filename = String::from(filename);
    let mut resized = ethumb;
    //let mut resized = resize_image(&thumb, cache_size_w, cache_size_h, filename.clone())?;
    if load_pred {
        resized = ops::affine(
            &resized,
            data.cache_size.w as f64 / cache_size_w as f64,
            0.,
            0.,
            data.cache_size.h as f64 / cache_size_h as f64,
        )
        .map_err(|err| ErrorKind::VipsOpError(filename.clone(), err.to_string()).into())?;
    }
    let vals = resized.image_write_to_memory();
    return Ok(vals);
}

/// Update the current extents, coordinates etc. with a given ZoomableImageViewer instance (:=
/// viewer). Used once a zoom button is clicked or the cache is updated after dragging far enough.
///
/// ```
/// # use slideslib::{ZoomableImageViewer, CACHE_MAX, WIDTH, HEIGHT};
/// # use slideslib::cache::update_zoom_props;
/// # use iced::application::Application;
/// # use std::path::PathBuf;
/// # use std::vec::Vec;
/// # use slideslib::error::ErrorKind;
/// # use openslide_rs::Size;
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.image_path = Vec::from([PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.tiff")]);
/// viewer.current_image = 0;
/// viewer.levels = Vec::from([1., 4., 16.]);
/// viewer.level = 16;
/// viewer.max_level = 16;
/// viewer.max_extents = Size{w: 1376 * 16, h: 288 * 16};
/// viewer.plot_data.view.cache_size.w = 512;
/// viewer.plot_data.view.cache_size.h = 512;
/// viewer.cache_scale_factor_x = 1.;
/// viewer.cache_scale_factor_y = 1.;
/// // Check if initial level leads to cache of maximum image size (thumbnail).
/// let err = update_zoom_props(&mut viewer);
/// assert!(matches!(err, None), "update_zoom_props failed with error {}", err.unwrap().to_string());
/// //assert_eq!(viewer.plot_data.view.cache_size.w, 100);
/// assert_eq!(viewer.tracker.preload_possible, false);
/// assert_eq!(viewer.current_extents.w, 1376);
/// assert_eq!(viewer.current_extents.h, 288);
/// assert_eq!(viewer.plot_data.view.viewport_size.w, 1376);
/// assert_eq!(viewer.plot_data.view.viewport_size.h, 288);
/// assert_eq!(viewer.plot_data.view.cache_size.w, 1376);
/// assert_eq!(viewer.plot_data.view.cache_size.h, 288);
/// assert_eq!(viewer.tracker.cache_size_x, 1376);
/// assert_eq!(viewer.tracker.cache_size_y, 288);
///
/// // Check if zooming changes the level and cache accordingly.
///
/// viewer.level = 14;
/// update_zoom_props(&mut viewer);
/// assert_eq!(viewer.level, 14);
/// assert_eq!(viewer.plot_data.view.viewport_size.w, 1204);
/// assert_eq!(viewer.plot_data.view.viewport_size.h, 287);
/// assert_eq!(viewer.plot_data.view.cache_size.w, 1376);
/// assert_eq!(viewer.plot_data.view.cache_size.h, 288);
/// assert_eq!(viewer.tracker.cache_size_x, 1376);
/// assert_eq!(viewer.tracker.cache_size_y, 288);
///
/// // // Check if viewport default and shrinked size were kept and new viewport size is adapted upon reaching a level
/// // // big enough for caching.
/// viewer.level = 1;
/// viewer.cache_scale_factor_x = 2.;
/// viewer.cache_scale_factor_y = 2.;
/// update_zoom_props(&mut viewer);
/// assert_eq!(viewer.tracker.preload_possible, true);
/// assert_eq!(viewer.plot_data.view.viewport_size.w, CACHE_MAX as u32 / 2);
/// assert_eq!(viewer.plot_data.view.viewport_size.h, (CACHE_MAX / 2. * HEIGHT as f32 / WIDTH as f32) as u32);
/// assert_eq!(viewer.plot_data.view.cache_size.w, CACHE_MAX as u32);
/// assert_eq!(viewer.plot_data.view.cache_size.h, (CACHE_MAX * HEIGHT as f32 / WIDTH as f32) as u32);
/// assert_eq!(viewer.plot_data.view.viewport_default.w, 1376);
/// assert_eq!(viewer.plot_data.view.viewport_default.h, 288);
/// assert_eq!(viewer.tracker.cache_size_x, CACHE_MAX as u32);
/// assert_eq!(viewer.tracker.cache_size_y, (CACHE_MAX * HEIGHT as f32 / WIDTH as f32) as u32);
/// assert_eq!(viewer.tracker.max_global_x, 22016.);
/// assert_eq!(viewer.tracker.min_global_x, 0.);
/// assert_eq!(viewer.tracker.max_cache_x,  CACHE_MAX as i32 / 2);
/// assert_eq!(viewer.tracker.min_cache_x, -(CACHE_MAX as i32) / 2);
/// assert_eq!(viewer.tracker.max_global_y, 4608.);
/// assert_eq!(viewer.tracker.min_global_y, 0.);
/// assert_eq!(viewer.tracker.max_cache_y, (CACHE_MAX / 2. * HEIGHT as f32 / WIDTH as f32) as i32);
/// assert_eq!(viewer.tracker.min_cache_y, -(CACHE_MAX / 2. * HEIGHT as f32 / WIDTH as f32) as i32);
///
/// # Ok::<(), ErrorKind>(())
/// ```
pub fn update_zoom_props(data: &mut ZoomableImageViewer) -> Option<ErrorKind> {
    if let Ok(slide) = OpenSlide::new(&data.image_path[data.current_image]) {
        let current_extents = slide
            .get_level_dimensions(0)
            .unwrap_or(OpenslideSize { w: 1024, h: 1024 });

        let (level_idx, level) =
            find_next_greater_value(data.levels.clone(), data.level).unwrap_or((0, data.level));
        let width = data.max_extents.w / level as u32;
        let height = data.max_extents.h / level as u32;
        data.tracker.preload_possible = false;
        data.current_zoom = data.level as f32 / level as f32;

        if (width as f32 > data.plot_data.view.cache_size.w as f32 / data.cache_scale_factor_x)
            | (height as f32 > data.plot_data.view.cache_size.h as f32 / data.cache_scale_factor_y)
        {
            data.tracker.preload_possible = true;
        }

        let shrinked_w = (current_extents.w / level) as i32;
        let shrinked_h = (current_extents.h / level) as i32;
        if data.level == data.max_level {
            data.plot_data.view.viewport_size = OpenslideSize {
                h: shrinked_h as u32,
                w: shrinked_w as u32,
            };
            data.cache_scale_factor_x = 1.;
            data.plot_data.view.cache_scale_factor_x = 1.;
            data.tracker.cache_scale_factor_x = 1.;
            data.plot_data.view.cache_size = OpenslideSize {
                w: (data.plot_data.view.viewport_size.w as f32 * data.cache_scale_factor_x) as u32,
                h: (data.plot_data.view.viewport_size.h as f32 * data.cache_scale_factor_y) as u32,
            };
            data.plot_data.view.viewport_default = data.plot_data.view.viewport_size;
            data.tracker.current_x = current_extents.w as f32 / 2.;
            data.tracker.current_y = current_extents.h as f32 / 2.;
        }
        let mut sfy;
        let sfx;
        let mut ignore_cache = true;
        data.plot_data.view.viewport_size = data.plot_data.view.viewport_default;
        let mag = data.max_level as f32 / data.level as f32;
        if data.plot_data.view.viewport_default.h as f32 * mag
            < (data.plot_data.view.global_height as f32 * 2.)
        {
            data.plot_data.view.viewport_size.h =
                (data.plot_data.view.viewport_default.h as f32 * mag) as u32;
            sfy = 1.;
            data.tracker.preload_possible = false;
        } else {
            data.tracker.preload_possible = true;
            data.plot_data.view.viewport_size.h = data.plot_data.view.global_height * 2;
            let cache_height = data.plot_data.view.viewport_default.h as f32 * mag;
            sfy = cache_height / (data.plot_data.view.global_height as f32 * 2.);
            ignore_cache = false;
            if sfy < 2. {
                ignore_cache = true;
                data.tracker.preload_possible = false;
            }
        }

        if data.plot_data.view.viewport_default.h as f32 * mag >= CACHE_MAX {
            data.plot_data.view.viewport_size.h = CACHE_MAX as u32 / 2;
            sfy = 2.;
        }

        data.plot_data.view.cache_size = data.plot_data.view.viewport_default;
        if level_idx != slide.get_level_count().unwrap_or(0) - 1 {
            data.plot_data.view.viewport_size.w =
                (data.plot_data.view.viewport_default.w as f32 * mag) as u32;

            if (data.plot_data.view.viewport_size.w as f32 >= (CACHE_MAX as f32 / 2.))
                & !ignore_cache
            {
                data.plot_data.view.viewport_size.w = CACHE_MAX as u32 / 2;
                sfx = 2.;
            } else {
                data.plot_data.view.viewport_size.w = data.plot_data.view.global_width * 2;
                let cache_width = data.plot_data.view.viewport_default.w as f32 * mag;
                sfx = cache_width / (data.plot_data.view.global_width as f32 * 2.);
            }
            let ratio_orig = data.plot_data.view.viewport_default.h as f32
                / data.plot_data.view.viewport_default.w as f32;
            let ratio = data.plot_data.view.viewport_size.h as f32
                / data.plot_data.view.viewport_size.w as f32;
            if (ratio != ratio_orig) & !ignore_cache {
                let window_ratio = data.plot_data.view.global_width as f32
                    / data.plot_data.view.global_height as f32;
                data.plot_data.view.viewport_size.h =
                    (data.plot_data.view.viewport_size.w as f32 / window_ratio) as u32;
            }
            data.plot_data.view.cache_scale_factor_x = sfx;
            data.cache_scale_factor_x = sfx;
            data.tracker.cache_scale_factor_x = sfx;
            data.plot_data.view.cache_scale_factor_y = sfy;
            data.cache_scale_factor_y = sfy;
            data.tracker.cache_scale_factor_y = sfy;
            data.plot_data.view.cache_size = OpenslideSize {
                w: (data.plot_data.view.viewport_size.w as f32 * data.cache_scale_factor_x) as u32,
                h: (data.plot_data.view.viewport_size.h as f32 * data.cache_scale_factor_y) as u32,
            };
        } else {
            data.tracker.preload_possible = false;
        }
        if data.level / level != 1 {
            data.plot_data.view.viewport_size.h =
                (data.plot_data.view.viewport_size.h as f32 * data.current_zoom) as u32;

            data.plot_data.view.viewport_size.w =
                (data.plot_data.view.viewport_size.w as f32 * data.current_zoom) as u32;
        }

        data.cache_scale_factor_y =
            data.plot_data.view.cache_size.h as f32 / data.plot_data.view.viewport_size.h as f32;
        data.cache_scale_factor_x =
            data.plot_data.view.cache_size.w as f32 / data.plot_data.view.viewport_size.w as f32;

        data.plot_data.view.cache_scale_factor_y =
            data.plot_data.view.cache_size.h as f32 / data.plot_data.view.viewport_size.h as f32;
        data.plot_data.view.cache_scale_factor_x =
            data.plot_data.view.cache_size.w as f32 / data.plot_data.view.viewport_size.w as f32;

        data.tracker.cache_scale_factor_y =
            data.plot_data.view.cache_size.h as f32 / data.plot_data.view.viewport_size.h as f32;
        data.tracker.cache_scale_factor_x =
            data.plot_data.view.cache_size.w as f32 / data.plot_data.view.viewport_size.w as f32;

        data.current_extents.w = current_extents.w / level;
        data.current_extents.h = current_extents.h / level;
        data.tracker.cache_comp_x = data.current_zoom;
        data.tracker.cache_comp_y = data.current_zoom;

        data.tracker.cache_size_x = data.plot_data.view.cache_size.w;
        data.tracker.cache_size_y = data.plot_data.view.cache_size.h;
        data.tracker.max_global_x = current_extents.w as f32;
        data.tracker.min_global_x = 0.;
        data.tracker.max_global_y = current_extents.h as f32;
        data.tracker.min_global_y = 0.;
        data.tracker.max_cache_x =
            data.plot_data.view.cache_size.w as i32 / data.cache_scale_factor_x as i32;
        data.tracker.min_cache_x =
            -1 * data.plot_data.view.cache_size.w as i32 / data.cache_scale_factor_x as i32;
        data.tracker.max_cache_y =
            data.plot_data.view.cache_size.h as i32 / data.cache_scale_factor_y as i32;
        data.tracker.min_cache_y =
            -1 * data.plot_data.view.cache_size.h as i32 / data.cache_scale_factor_y as i32;

        let mppx = slide
            .get_property_value(OPENSLIDE_PROPERTY_NAME_MPP_X)
            .unwrap_or(String::from("0."))
            .parse::<f32>()
            .unwrap_or(0.);
        data.mppx = slide
            .get_all_level_downsample()
            .unwrap_or(Vec::from([1.]))
            .into_iter()
            .map(|x| mppx * (x as f32))
            .collect();
        let mppy = slide
            .get_property_value(OPENSLIDE_PROPERTY_NAME_MPP_Y)
            .unwrap_or(String::from("0."))
            .parse::<f32>()
            .unwrap_or(0.);
        data.mppy = slide
            .get_all_level_downsample()
            .unwrap_or(Vec::from([1.]))
            .iter()
            .map(|y| mppy * *y as f32)
            .collect();
        return None;
    } else {
        return Some(ErrorKind::OpenSlidePropertiesError(
            data.image_path[data.current_image].clone(),
        ));
    }
}

pub fn update_cache_data(
    data: &mut ZoomableImageViewer,
    background: bool,
    imagetype: ImageType,
) -> Option<ErrorKind> {
    match imagetype {
        ImageType::WSI => update_wsi_cache_data(data, background),
        _ => match update_dicom_cache_data(data, background) {
            Ok(_) => None,
            Err(err) => Some(err),
        },
    }
}
/// Update the cache for WSI images. Can be run in foreground or a separate thread in the background.
///
/// Example:
///
/// ```
/// # use slideslib::{ZoomableImageViewer, cache::update_zoom_props};
/// # use slideslib::cache::update_wsi_cache_data;
/// # use iced::application::Application;
/// # use std::path::PathBuf;
/// # use std::vec::Vec;
/// # use slideslib::error::ErrorKind;
/// # use openslide_rs::Size;
/// # use std::sync::Arc;
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.image_path = Vec::from(
/// [PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.tiff")]
/// );
/// viewer.current_image = 0;
/// viewer.levels = Vec::from([1., 4., 16.]);
/// viewer.level = 16;
/// viewer.max_level = 16;
/// viewer.max_extents = Size{w: 1376 * 16, h: 288 * 16};
/// viewer.cache_scale_factor_x = 1.;
/// viewer.cache_scale_factor_y = 1.;
/// // Check if initial level leads to cache of maximum image size (thumbnail).
/// # let err = update_zoom_props(&mut viewer);
/// let err = update_wsi_cache_data(&mut viewer, false);
/// assert!(matches!(err, None), "update_cache_data failed for WSI with error {}", err.unwrap().to_string());
/// let cache_arc = Arc::clone(&viewer.loadtime_cache);
/// let cache = cache_arc.lock().unwrap();
/// let cache_data = cache.clone().borrow().to_vec();
/// let sum: u32 = cache_data.iter().map(|x| *x as u32).sum();
/// assert_ne!(sum, 0);
/// # Ok::<(), ErrorKind>(())
/// ```
pub fn update_wsi_cache_data(
    data: &mut ZoomableImageViewer,
    background: bool,
) -> Option<ErrorKind> {
    let load_pred = data.show_pred;

    let path = data.image_path[data.current_image].to_str().unwrap_or("");
    let impath = replace_suffix_with_pred(path);
    let path = String::from(path);
    if background {
        let loadtime_cache_arc = Arc::clone(&data.loadtime_cache);
        let update_ready_arc = Arc::clone(&data.update_ready);
        let preload_args: PreloadRegionArgs = data.into();
        let thread_error_arc = Arc::clone(&data.load_thread_error);
        // Spawn a new thread to modify 'bar' in the background
        thread::spawn(move || {
            // Access the shared data
            let mut update_ready = match update_ready_arc.lock() {
                Ok(val) => val,
                Err(err) => {
                    log_or_load_thread_err(
                        thread_error_arc,
                        Some(ErrorKind::ThreadError(err.to_string())),
                    );
                    return;
                }
            };
            *update_ready = false;
            let loadtime_cache = match loadtime_cache_arc.lock() {
                Ok(val) => val,
                Err(err) => {
                    log_or_load_thread_err(
                        thread_error_arc,
                        Some(ErrorKind::ThreadError(err.to_string())),
                    );
                    return;
                }
            };
            match (
                get_region(preload_args.clone(), false, path.clone()),
                if load_pred & PathBuf::from(impath.clone()).exists() {
                    get_region(preload_args, true, impath.clone())
                } else {
                    Ok(Vec::new())
                },
            ) {
                (Ok(img_region), Ok(pred_region)) => {
                    let mut region = img_region;
                    if load_pred {
                        region = region
                            .iter()
                            .zip(pred_region.iter())
                            .map(|(&i, &p)| ((i as f32 * 0.35) + ((p) as f32 * 0.65)) as u8)
                            .collect();
                    }
                    loadtime_cache.replace(region);
                    *update_ready = true;
                }
                (Ok(region), Err(err)) => {
                    log_or_load_thread_err(
                        thread_error_arc,
                        Some(ErrorKind::ThreadError(err.to_string())),
                    );
                    loadtime_cache.replace(region);
                    *update_ready = true;
                }
                (Err(err), Err(err2)) => {
                    log_or_load_thread_err(
                        thread_error_arc,
                        Some(ErrorKind::ThreadMultiError(
                            err.to_string(),
                            err2.to_string(),
                        )),
                    );
                }
                _ => {}
            };
        });
        return None;
    }

    let preload_args: PreloadRegionArgs = data.into();
    let success_or_fail = match (
        get_region(preload_args.clone(), false, path.clone()),
        if load_pred & PathBuf::from(impath.clone()).exists() {
            get_region(preload_args, true, impath.clone())
        } else {
            Ok(Vec::new())
        },
    ) {
        (Ok(img_region), Ok(pred_region)) => {
            let mut region = img_region;
            if load_pred {
                region = region
                    .iter()
                    .zip(pred_region.iter())
                    .map(|(&i, &p)| ((i as f32 * 0.35) + ((p) as f32 * 0.65)) as u8)
                    .collect();
            }
            data.plot_data.view.cache.replace(region);
            None
        }
        (Ok(region), Err(err)) => {
            data.plot_data.view.cache.replace(region);
            if load_pred {
                return Some(ErrorKind::MaskLoadingError(
                    path.clone(),
                    err.to_string().to_string(),
                ));
            }
            None
        }
        (Err(err), Err(err2)) => {
            return Some(ErrorKind::BothLoadingError(
                path.clone(),
                err.to_string(),
                err2.to_string(),
            ));
        }
        _ => None,
    };
    success_or_fail
}

/// Update the cache for dicom data.
///
/// Example:
///
/// ```
/// # use slideslib::{ZoomableImageViewer, cache::update_zoom_props};
/// # use slideslib::cache::update_dicom_cache_data;
/// # use iced::application::Application;
/// # use std::path::PathBuf;
/// # use std::vec::Vec;
/// # use slideslib::error::ErrorKind;
/// # use openslide_rs::Size;
/// # use std::sync::Arc;
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.image_path = Vec::from(
/// [PathBuf::from("data").join("preprocessed").join("MRI Test")]
/// );
/// viewer.current_image = 0;
/// viewer.levels = Vec::from([1., 4., 16.]);
/// viewer.level = 16;
/// viewer.max_level = 22;
/// viewer.max_extents = Size{w: 768, h: 256};
/// viewer.cache_scale_factor_x = 1.;
/// viewer.cache_scale_factor_y = 1.;
/// // Check if initial level leads to cache of maximum image size (thumbnail).
/// let err = update_dicom_cache_data(&mut viewer, false);
/// assert!(err.is_ok());
/// //assert!(matches!(err, None), "update_cache_data failed for DCM.");
/// let cache_data = &viewer.plot_data.view.cache.borrow();
/// let sum: &u32 = &cache_data.iter().map(|x| *x as u32).sum();
/// assert_ne!(*sum, 0);
/// # Ok::<(), ErrorKind>(())
/// ```
pub fn update_dicom_cache_data(
    data: &mut ZoomableImageViewer,
    _background: bool,
) -> Result<(), ErrorKind> {
    let load_pred = data.show_pred;
    let path_ = &data
        .image_path
        .get(data.current_image)
        .ok_or(ErrorKind::NoFileError())?
        .join("whole.npy");
    let pred_path = &data
        .image_path
        .get(data.current_image)
        .ok_or(ErrorKind::NoFileError())?
        .join("pred.npy");
    println!("{:?} {:?} {:?}", &data.image_path, path_, pred_path);

    // Load input image
    let bytes = std::fs::read(path_)
        .map_err(|_| ErrorKind::DicomImageLoadingError(PathBuf::from(path_)))?;
    let numpy_data = npyz::NpyFile::new(&bytes[..])
        .map_err(|_| ErrorKind::DicomImageLoadingError(PathBuf::from(path_)))?
        .into_vec::<f32>()
        .map_err(|_| ErrorKind::DicomImageLoadingError(PathBuf::from(path_)))?;

    data.plot_data.view.cache.replace(
        numpy_data
            .clone()
            .iter()
            .flat_map(|f| f.to_ne_bytes()) // Convert each f32 to an numpy_data of 4 bytes
            .collect(),
    );
    data.max_level = (numpy_data.to_vec().len() / (224 * 224 * 3)) as u32 - 1;

    // Load prediction data
    if load_pred & pred_path.exists() {
        let bytes = std::fs::read(pred_path)
            .map_err(|_| ErrorKind::DicomImageLoadingError(PathBuf::from(pred_path)))?;

        // Note: In addition to byte slices, this accepts any io::Read
        let pred_data = npyz::NpyFile::new(&bytes[..])
            .map_err(|_| ErrorKind::DicomImageLoadingError(PathBuf::from(pred_path)))?
            .into_vec::<f32>()
            .map_err(|_| ErrorKind::DicomImageLoadingError(PathBuf::from(pred_path)))?;
        data.plot_data.view.mask_cache.replace(
            pred_data
                .clone()
                .iter()
                .flat_map(|f| f.to_ne_bytes()) // Convert each f32 to an numpy_data of 4 bytes
                .collect(),
        );
    }
    Ok(())
}

/// Change the cache data of the plotter after it has been updated. Can be used for both background
/// and non-background updates.
///
/// Example:
///
/// ```
/// # use slideslib::{ZoomableImageViewer, cache::change_cache};
/// # use slideslib::cache::update_cache_data;
/// # use iced::application::Application;
/// # use std::path::PathBuf;
/// # use std::vec::Vec;
/// # use slideslib::error::ErrorKind;
/// # use openslide_rs::Size;
/// # use std::sync::Arc;
///
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.image_path = Vec::from(
///     [PathBuf::from("tests").join("02a7b258e875cf073e2421d67ff824cd.tiff")]
/// );
/// {
///     let mock_vec = Vec::from([0, 1, 2]);
///     let cache_arc = Arc::clone(&viewer.loadtime_cache);
///     let mut cache = cache_arc.lock().unwrap();
///     //let cache_data = cache.clone().borrow().to_vec();
///     cache.replace(mock_vec);
///     let update_ready_arc = Arc::clone(&viewer.update_ready);
///     let mut update_ready = update_ready_arc.lock().unwrap();
///     *update_ready = true;
/// }
/// change_cache(&mut viewer, false);
/// let sum: u32 = viewer.plot_data.view.cache.borrow().iter().map(|x| *x as u32).sum();
/// assert_ne!(sum, 0);
/// ```
pub fn change_cache(data: &mut ZoomableImageViewer, _background: bool) -> Option<String> {
    let update_ready_data = Arc::clone(&data.update_ready);
    let _update_ready;
    if let Ok(val) = update_ready_data.lock() {
        _update_ready = val
    } else {
        return Some(ErrorKind::ChacheChangingError(String::from("runtime")).to_string());
    };
    //while !*update_ready & background {
    //    println!("Waiting");
    //}

    let loadtime_cache_data = Arc::clone(&data.loadtime_cache);
    let loadtime_data;
    if let Ok(val) = loadtime_cache_data.lock() {
        loadtime_data = val
    } else {
        return Some(ErrorKind::ChacheChangingError(String::from("loadtime")).to_string());
    };

    let vec = loadtime_data.clone();
    let vec_data = vec.clone().borrow().to_vec();
    data.plot_data.view.cache.replace(vec_data);
    return None;
}

/// Reset the current offsets (x, y) of the slide viewer.
///
/// Example:
///
/// ```
/// # use slideslib::{ZoomableImageViewer, cache::reset_offsets, cache::Border, tracking::Borders};
/// # use slideslib::error::ErrorKind;
/// # use iced::application::Application;
///
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// let orig_offsetx = viewer.offsetx;
/// let orig_offsety = viewer.offsety;
/// let orig_tracker_current_x = viewer.tracker.current_x;
/// let orig_tracker_current_y = viewer.tracker.current_y;
/// let orig_plot_data_cache_posx = viewer.plot_data.view.cache_posx;
/// let orig_plot_data_cache_posy = viewer.plot_data.view.cache_posy;
/// let orig_current_border = viewer.current_border;
/// let orig_center_correction_x = viewer.tracker.center_correction_x;
/// let orig_center_correction_y = viewer.tracker.center_correction_y;
///
/// viewer.offsetx = 12345.;
/// viewer.offsety = 12345.;
/// viewer.tracker.current_x = 12345.;
/// viewer.tracker.current_y = 12345.;
/// viewer.plot_data.view.cache_posx = 12345.;
/// viewer.plot_data.view.cache_posy = 12345.;
/// viewer.current_border = Border {cache: Borders::Left, edge: Borders::Left};
/// viewer.tracker.center_correction_x = 12345.;
/// viewer.tracker.center_correction_y = 12345.;
///
/// reset_offsets(&mut viewer);
///
/// // Current position is (256, 256) due to 512 max cache size
/// assert_eq!(256., viewer.offsetx);
/// assert_eq!(256., viewer.offsety);
/// assert_eq!(256., viewer.tracker.current_x);
/// assert_eq!(256., viewer.tracker.current_y);
/// assert_eq!(orig_plot_data_cache_posx, viewer.plot_data.view.cache_posx);
/// assert_eq!(orig_plot_data_cache_posy, viewer.plot_data.view.cache_posy);
/// assert_eq!(orig_current_border.cache, viewer.current_border.cache);
/// assert_eq!(orig_current_border.edge, viewer.current_border.edge);
/// assert_eq!(orig_center_correction_x, viewer.tracker.center_correction_x);
/// assert_eq!(orig_center_correction_y, viewer.tracker.center_correction_y);
/// ```
pub fn reset_offsets(data: &mut ZoomableImageViewer) {
    data.offsetx = data.max_extents.w as f32 / 2.;
    data.offsety = data.max_extents.h as f32 / 2.;
    data.tracker.current_x = data.offsetx;
    data.tracker.current_y = data.offsety;
    data.plot_data.view.cache_posx = 0.;
    data.plot_data.view.cache_posy = 0.;
    data.current_border = Border {
        cache: Borders::Center,
        edge: Borders::Center,
    };
    data.tracker.center_correction_x = 0.;
    data.tracker.center_correction_y = 0.;
}

/// Update all offsets after dragging the image far enough according to the new position.
/// Coordinates will be clipped according to the global extents and currently specified
/// cache size (slideslib::MAX_CACHE) to ensure valid coordinates and avoid runtime errors.
///
/// Example:
///
/// ```
/// # use slideslib::{ZoomableImageViewer, cache::update_offsets, cache::Border, tracking::Borders};
/// # use slideslib::error::ErrorKind;
/// # use iced::application::Application;
/// # use std::vec::Vec;
/// # use slideslib::CACHE_MAX;
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.tracker.preload_possible = true;
/// viewer.levels = Vec::from([1., 2.]);
/// viewer.level = 2;
/// viewer.max_extents.w = (CACHE_MAX * 8.) as u32;
/// viewer.max_extents.h = (CACHE_MAX * 8.) as u32;
/// viewer.offsetx = CACHE_MAX * 4.;
/// viewer.offsety = CACHE_MAX * 4.;
/// viewer.tracker.current_x = CACHE_MAX * 4.;
/// viewer.tracker.current_y = CACHE_MAX * 4.;
///
/// viewer.tracker.cache_size_x = CACHE_MAX as u32;
/// viewer.tracker.cache_size_y = CACHE_MAX as u32;
/// viewer.tracker.max_global_x = CACHE_MAX * 8.;
/// viewer.tracker.min_global_x = 0.;
/// viewer.tracker.max_global_y = CACHE_MAX * 8.;
/// viewer.tracker.min_global_y = 0.;
/// viewer.tracker.max_cache_x = CACHE_MAX as i32;
/// viewer.tracker.min_cache_x = -1 * CACHE_MAX as i32;
/// viewer.tracker.max_cache_y = CACHE_MAX as i32;
/// viewer.tracker.min_cache_y = -1 * CACHE_MAX as i32;
///
/// // Position update within global extents is accepted. 100 is added.
/// viewer.plot_data.view.cache_posx = 50.;
/// viewer.plot_data.view.cache_posy = 50.;
/// update_offsets(&mut viewer, 2);
/// assert_eq!(viewer.offsetx, 2148.);
/// assert_eq!(viewer.offsety, 2148.);
/// assert_eq!(viewer.plot_data.view.cache_posx, 0.);
/// assert_eq!(viewer.plot_data.view.cache_posy, 0.);
/// assert_eq!(viewer.tracker.current_x, 2148.);
/// assert_eq!(viewer.tracker.current_y, 2148.);
/// # viewer.plot_data.view.cache_posx = -50.;
/// # viewer.plot_data.view.cache_posy = -50.;
/// # update_offsets(&mut viewer, 2);
/// # assert_eq!(viewer.offsetx, 2048.);
/// # assert_eq!(viewer.offsety, 2048.);
/// # assert_eq!(viewer.plot_data.view.cache_posx, 0.);
/// # assert_eq!(viewer.plot_data.view.cache_posy, 0.);
/// # assert_eq!(viewer.tracker.current_x, 2048.);
/// # assert_eq!(viewer.tracker.current_y, 2048.);
///
/// // Position update outside the global extents is clipped. Distance if 1024 in both max and 0
/// // direction.
/// viewer.plot_data.view.cache_posx = 50000.;
/// viewer.plot_data.view.cache_posy = -50000.;
/// update_offsets(&mut viewer, 2);
/// assert_eq!(viewer.offsetx, 4096.);
/// assert_eq!(viewer.offsety, 0.);
///
/// // Zooming out causes smaller steps in position update. Steps in multiple levels can be added.
/// viewer.plot_data.view.cache_posx = -50.;
/// viewer.plot_data.view.cache_posy = 50.;
/// viewer.level = 1;
/// update_offsets(&mut viewer, 1);
/// assert_eq!(viewer.offsetx, 4046.);
/// assert_eq!(viewer.offsety, 50.);
///
/// // Without preloading, position update outside cache extents is clipped.
/// viewer.tracker.preload_possible = false;
/// viewer.plot_data.view.cache_posx = 1000.;
/// viewer.plot_data.view.cache_posy = -1000.;
/// viewer.level = 2;
/// update_offsets(&mut viewer, 2);
/// assert_eq!(viewer.plot_data.view.cache_posx, 128.);
/// assert_eq!(viewer.plot_data.view.cache_posy, -128.);
///
/// // Reaching a border will cause reload and thus reseting the current border to 'Center', except
/// // the case that the outermost point is reached.
/// viewer.current_border.cache = Borders::Bottom;
/// viewer.current_border.edge = Borders::Bottom;
/// update_offsets(&mut viewer, 2);
/// assert_eq!(viewer.current_border.cache, Borders::Center);
/// assert_eq!(viewer.current_border.edge, Borders::Center);
/// viewer.current_border.cache = Borders::BottomLimit;
/// update_offsets(&mut viewer, 2);
/// assert_ne!(viewer.current_border.cache, Borders::Center);
pub fn update_offsets(data: &mut ZoomableImageViewer, old_level: u32) {
    data.tracker.current_x = data.offsetx;
    data.tracker.current_y = data.offsety;
    let (_, level) =
        find_next_greater_value(data.levels.clone(), data.level).unwrap_or((0, data.level));
    let (_, old_greater_level) =
        find_next_greater_value(data.levels.clone(), old_level).unwrap_or((0, old_level));
    let old_zoom = old_level as f32 / old_greater_level as f32;
    let is_edge = match data.current_border.cache {
        Borders::BottomLimit
        | Borders::BottomLeftLimit
        | Borders::BottomRightLimit
        | Borders::TopLimit
        | Borders::TopLeftLimit
        | Borders::TopRightLimit
        | Borders::LeftLimit
        | Borders::RightLimit => true,
        _ => false,
    };

    let viewport_size_x = (data.plot_data.view.cache_size.w as f32 / 2.) * old_zoom;
    let viewport_size_y = (data.plot_data.view.cache_size.h as f32 / 2.) * old_zoom;
    let cache_size_x = data.plot_data.view.cache_size.w as f32;
    let cache_size_y = data.plot_data.view.cache_size.h as f32;
    let cache_correction_x = (cache_size_x / 2. - viewport_size_x / 2.) / 2.;
    let cache_correction_y = (cache_size_y / 2. - viewport_size_y / 2.) / 2.;
    if old_greater_level != data.max_level {
        if data.plot_data.view.cache_posx as f32 > cache_correction_x {
            data.offsetx -= cache_correction_x * 2. * old_greater_level as f32;
        }
        if (data.plot_data.view.cache_posx as f32) < (-1. * cache_correction_x) {
            data.offsetx += cache_correction_x * 2. * old_greater_level as f32;
        }
        if data.plot_data.view.cache_posy as f32 > cache_correction_y {
            data.offsety -= cache_correction_y * 2. * old_greater_level as f32;
        }
        if (data.plot_data.view.cache_posy as f32) < (-1. * cache_correction_y) {
            data.offsety += cache_correction_y * 2. * old_greater_level as f32;
        }
    }
    if data.tracker.preload_possible {
        if old_greater_level != data.max_level {
            data.offsetx = data.offsetx + data.plot_data.view.cache_posx * old_greater_level as f32;
            data.offsety = data.offsety + data.plot_data.view.cache_posy * old_greater_level as f32;
        } else {
            data.offsetx = data.max_extents.w as f32 / 2.
                + data.plot_data.view.cache_posx * old_greater_level as f32;
            data.offsety = data.max_extents.h as f32 / 2.
                + data.plot_data.view.cache_posy * old_greater_level as f32;
        }
        data.plot_data.view.cache_posx = 0.;
        data.plot_data.view.cache_posy = 0.;
    } else {
        let mut correction_x = 0.;
        let mut correction_y = 0.;

        if old_greater_level != data.max_level {
            correction_x = data.max_extents.w as f32 / 2. - data.offsetx;
            correction_y = data.max_extents.h as f32 / 2. - data.offsety;
        }
        data.plot_data.view.cache_posx = (correction_x
            - data.plot_data.view.cache_posx * old_greater_level as f32)
            / level as f32;
        data.plot_data.view.cache_posy = (correction_y
            - data.plot_data.view.cache_posy / old_greater_level as f32)
            / level as f32;
        data.offsetx = data.max_extents.w as f32 / 2.;
        data.offsety = data.max_extents.h as f32 / 2.;
        data.plot_data.view.cache_posx *= -1.;
        data.plot_data.view.cache_posy *= -1.;
        data.tracker.clip_cache_coords(
            &mut data.plot_data.view.cache_posx,
            &mut data.plot_data.view.cache_posy,
        );
    }
    data.tracker
        .clip_global_coords(&mut data.offsetx, &mut data.offsety, level);

    if !is_edge {
        data.current_border = Border {
            cache: Borders::Center,
            edge: Borders::Center,
        };
    }
    data.tracker.center_correction_x = 0.;
    data.tracker.center_correction_y = 0.;
    data.tracker.current_x = data.offsetx;
    data.tracker.current_y = data.offsety;
}
