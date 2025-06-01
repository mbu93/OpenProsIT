use std::cell::RefCell;
use std::rc::Rc;

use iced::{Point, Rectangle};

use image::math::Rect;

use ndarray::{s, ArrayBase, Ix1, Ix3, OwnedRepr};
use openslide_rs::Size as OpenslideSize;

pub struct BaseView {
    pub cache: Rc<RefCell<Vec<u8>>>,
    pub mask_cache: Rc<RefCell<Vec<u8>>>,
    pub viewport_size: OpenslideSize,
    pub viewport_default: OpenslideSize,
    pub cache_size: OpenslideSize,
    pub cache_posx: f32,
    pub cache_posy: f32,
    pub xoffset: Option<u16>,
    pub yoffset: Option<u16>,
    pub mask_active: bool,
    pub sel_start: Option<Point>,
    pub sel_end: Option<Point>,
    pub global_width: u32,
    pub global_height: u32,
    pub cache_scale_factor_x: f32,
    pub cache_scale_factor_y: f32,
}

pub struct BaseViewArgs {
    pub cache: Rc<RefCell<Vec<u8>>>,
    pub mask_cache: Rc<RefCell<Vec<u8>>>,
    pub viewport_size: OpenslideSize,
    pub viewport_default: OpenslideSize,
    pub cache_size: OpenslideSize,
    pub cache_posx: f32,
    pub cache_posy: f32,
    pub xoffset: Option<u16>,
    pub yoffset: Option<u16>,
    pub mask_active: bool,
    pub sel_start: Option<Point>,
    pub sel_end: Option<Point>,
    pub global_width: u32,
    pub global_height: u32,
    pub cache_scale_factor_x: f32,
    pub cache_scale_factor_y: f32,
}

impl BaseViewArgs {
    pub fn new(
        cache: Rc<RefCell<Vec<u8>>>,
        mask_cache: Rc<RefCell<Vec<u8>>>,
        viewport_size: OpenslideSize,
        viewport_default: OpenslideSize,
        cache_size: OpenslideSize,
        cache_posx: f32,
        cache_posy: f32,
        xoffset: Option<u16>,
        yoffset: Option<u16>,
        mask_active: bool,
        sel_start: Option<Point>,
        sel_end: Option<Point>,
        global_width: u32,
        global_height: u32,
        cache_scale_factor_x: f32,
        cache_scale_factor_y: f32,
    ) -> Self {
        Self {
            cache,
            mask_cache,
            viewport_size,
            viewport_default,
            cache_size,
            cache_posx,
            cache_posy,
            xoffset,
            yoffset,
            mask_active,
            sel_start,
            sel_end,
            global_width,
            global_height,
            cache_scale_factor_x,
            cache_scale_factor_y,
        }
    }
}

impl BaseView {
    /// Create a new BaseView Widget that can be used to render the WSI images. Takes the
    /// following arguments:
    ///
    /// - cache: the current image data cache
    /// - mask_cache: the current prediction overlay cache
    /// - viewport_size: visible area
    /// - cache_size: cache size
    /// - cache_posx: x position in cache
    /// - cache_posy: y position in cache
    /// - xoffset: x offset of the widget itself,
    /// - yoffset: y offset of the widget itself,
    /// - mask_active: activate prediction rendering,
    /// - sel_start: left top point of roi rectangle,
    /// - sel_end: right bottom point of roi rectangle,
    /// - global_width: UI width,
    /// - global_height: UI height,
    /// - cache_scale_factor_x: relation of x cache / viewport,
    /// - cache_scale_factor_y: relation of y cache / viewport,
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::{WIDTH, HEIGHT, renderer::BaseView};
    /// # use openslide_rs::Size;
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use ndarray::Array;
    /// let cs = 512; // cache size;
    /// let cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
    /// let mask_cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
    /// let cache = Rc::new(RefCell::new(cache_init));
    /// let mask_cache = Rc::new(RefCell::new(mask_cache_init));
    /// let cs = cs as u32;
    /// let plot_data = BaseView {
    ///     cache_posx: 0.0,
    ///     cache_posy: 0.0,
    ///     cache_size: Size { w: cs, h: cs },
    ///     viewport_size: Size {
    ///         w: cs / 2,
    ///         h: cs / 2,
    ///     },
    ///     viewport_default: Size {
    ///         w: cs / 2,
    ///         h: cs / 2,
    ///     },
    ///     xoffset: Some(200),
    ///     yoffset: Some(512),
    ///     cache,
    ///     mask_cache,
    ///     mask_active: false,
    ///     sel_start: None,
    ///     sel_end: None,
    ///     global_width: WIDTH,
    ///     global_height: HEIGHT,
    ///     cache_scale_factor_x: 2.,
    ///     cache_scale_factor_y: 2.,
    /// };
    /// ```
    pub fn new(args: BaseViewArgs) -> Self {
        Self {
            cache: args.cache,
            mask_cache: args.mask_cache,
            viewport_size: args.viewport_size,
            viewport_default: args.viewport_default,
            cache_size: args.cache_size,
            cache_posx: args.cache_posx,
            cache_posy: args.cache_posy,
            xoffset: args.xoffset,
            yoffset: args.yoffset,
            mask_active: args.mask_active,
            sel_start: args.sel_start,
            sel_end: args.sel_end,
            global_width: args.global_width,
            global_height: args.global_height,
            cache_scale_factor_x: args.cache_scale_factor_x,
            cache_scale_factor_y: args.cache_scale_factor_y,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct PositionDetails {
    pub width: usize,
    pub height: usize,
    pub hmax: f32,
    pub wmax: f32,
    pub yoffset: f32,
    pub xoffset: f32,
    pub bounds: Rect,
}

/// Get the current viewport bounding coordinates.
///
/// Example:
///
/// ```
/// # use slideslib::{WIDTH, HEIGHT, renderer::{BaseView, get_viewport_bounds}};
/// # use openslide_rs::Size;
/// # use std::cell::RefCell;
/// # use std::rc::Rc;
/// # use ndarray::Array;
/// # let cs = 512; // cache size;
/// # let cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
/// # let mask_cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
/// # let cache = Rc::new(RefCell::new(cache_init));
/// # let mask_cache = Rc::new(RefCell::new(mask_cache_init));
/// # let cs = cs as u32;
/// # let slideview = BaseView {
/// #    cache_posx: 0.0,
/// #    cache_posy: 0.0,
/// #    cache_size: Size { w: cs, h: cs },
/// #    viewport_size: Size {
/// #        w: cs / 2,
/// #        h: cs / 2,
/// #    },
/// #    viewport_default: Size {
/// #        w: cs / 2,
/// #        h: cs / 2,
/// #    },
/// #    xoffset: Some(200),
/// #    yoffset: Some(512),
/// #    cache,
/// #    mask_cache,
/// #    mask_active: false,
/// #    sel_start: None,
/// #    sel_end: None,
/// #    global_width: WIDTH,
/// #    global_height: HEIGHT,
/// #    cache_scale_factor_x: 2.,
/// #    cache_scale_factor_y: 2.
/// # };
/// let bounds = get_viewport_bounds(&slideview);
/// assert_eq!(bounds.x, 128);
/// assert_eq!(bounds.y, 128);
/// assert_eq!(bounds.width, 256);
/// assert_eq!(bounds.height, 256);
/// ```
pub fn get_viewport_bounds(data: &BaseView) -> Rect {
    let x0 = data.cache_size.w as f32 / 2. - data.viewport_size.w as f32 / 2. + data.cache_posx;
    let x1 = data.cache_size.w as f32 / 2. + data.viewport_size.w as f32 / 2. + data.cache_posx;
    let y0 = data.cache_size.h as f32 / 2. - data.viewport_size.h as f32 / 2. + data.cache_posy;
    let y1 = data.cache_size.h as f32 / 2. + data.viewport_size.h as f32 / 2. + data.cache_posy;

    Rect {
        x: x0 as u32,
        y: y0 as u32,
        width: (x1 - x0) as u32,
        height: (y1 - y0) as u32,
    }
}

pub fn draw_rect(
    flat_vec: &mut ArrayBase<OwnedRepr<u8>, Ix3>,
    bounds: Rectangle,
    c: Option<Vec<u8>>,
) {
    let y = bounds.y as usize;
    let x = bounds.x as usize;
    let w = bounds.width as usize;
    let h = bounds.height as usize;
    let c = c.unwrap_or(Vec::from([0, 0, 0]));
    flat_vec
        .slice_mut(s!(y..y + 5, x..x + w, 0..3))
        .assign(&ArrayBase::<OwnedRepr<u8>, Ix1>::from_vec(c.clone()));
    flat_vec
        .slice_mut(s!(y + h - 5..y + h, x..x + w, 0..3))
        .assign(&ArrayBase::<OwnedRepr<u8>, Ix1>::from_vec(c.clone()));
    flat_vec
        .slice_mut(s!(y..y + h, x..x + 5, 0..3))
        .assign(&ArrayBase::<OwnedRepr<u8>, Ix1>::from_vec(c.clone()));
    flat_vec
        .slice_mut(s!(y..y + h, x + w - 5..x + w, 0..3))
        .assign(&ArrayBase::<OwnedRepr<u8>, Ix1>::from_vec(c.clone()));
}

impl BaseView {
    /// Get information required for rendering.
    ///
    /// Example
    /// ```
    /// # use slideslib::{WIDTH, HEIGHT, renderer::{BaseView, get_viewport_bounds,
    ///                                             PositionDetails}};
    /// # use openslide_rs::Size;
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use ndarray::Array;
    /// # use iced::Point;
    /// # use image::math::Rect;
    /// # let cs = 512; // cache size;
    /// # let cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
    /// # let mask_cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
    /// # let cache = Rc::new(RefCell::new(cache_init));
    /// # let mask_cache = Rc::new(RefCell::new(mask_cache_init));
    /// # let cs = cs as u32;
    /// # let slideview = BaseView {
    /// #    cache_posx: 0.0,
    /// #    cache_posy: 0.0,
    /// #    cache_size: Size { w: cs, h: cs },
    /// #    viewport_size: Size {
    /// #        w: cs / 2,
    /// #        h: cs / 2,
    /// #    },
    /// #    viewport_default: Size {
    /// #        w: cs / 2,
    /// #        h: cs / 2,
    /// #    },
    /// #    xoffset: Some(200),
    /// #    yoffset: Some(512),
    /// #    cache,
    /// #    mask_cache,
    /// #    mask_active: false,
    /// #    sel_start: Some(Point {x: 0., y: 0.}),
    /// #    sel_end: Some(Point {x: 100., y: 100.}),
    /// #    global_width: WIDTH,
    /// #    global_height: HEIGHT,
    /// #    cache_scale_factor_y: 2.,
    /// #    cache_scale_factor_x: 2.,
    /// # };
    /// // For a selection from (0, 0) to (10, 10)
    /// let details = slideview.get_position_details();;
    ///
    /// let val = PositionDetails { width: 256, height: 256, hmax: 560.0, wmax: 600.0,
    ///                             yoffset: 40.0, xoffset: 200.0,
    ///                             bounds: Rect { x: 128, y: 128, width: 256, height: 256 } };
    /// assert_eq!(details, val);
    ///
    /// Ok::<(), &'static str>(())
    /// ```
    pub fn get_position_details(&self) -> PositionDetails {
        let bounds = get_viewport_bounds(self);
        let width = bounds.width as usize;
        let height = bounds.height as usize;
        let max_width = self.global_width as f32 - self.xoffset.unwrap_or(0) as f32;
        let max_height = self.global_height as f32 - 40.;

        let mut hmax = height as f32 * max_width as f32 / width as f32 - 40.;
        let mut wmax = max_width;
        if hmax > max_height {
            wmax = max_width * max_height / hmax as f32;
            hmax = max_height;
        }
        let xoffset = self.xoffset.unwrap_or(0) as f32;
        let yoffset = 40. + (self.global_height as f32 - 40.) / 2. - hmax / 2.;
        return PositionDetails {
            width,
            height,
            hmax,
            wmax,
            xoffset,
            yoffset,
            bounds,
        };
    }

    /// Get the bounding positions of the current selection.
    ///
    /// Example
    /// ```
    /// # use slideslib::{WIDTH, HEIGHT, renderer::{BaseView, get_viewport_bounds}};
    /// # use openslide_rs::Size;
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use ndarray::Array;
    /// # use iced::Point;
    /// # let cs = 512; // cache size;
    /// # let cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
    /// # let mask_cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();
    /// # let cache = Rc::new(RefCell::new(cache_init));
    /// # let mask_cache = Rc::new(RefCell::new(mask_cache_init));
    /// # let cs = cs as u32;
    /// # let slideview = BaseView {
    /// #    cache_posx: 0.0,
    /// #    cache_posy: 0.0,
    /// #    cache_size: Size { w: cs, h: cs },
    /// #    viewport_size: Size {
    /// #        w: cs / 2,
    /// #        h: cs / 2,
    /// #    },
    /// #    viewport_default: Size {
    /// #        w: cs / 2,
    /// #        h: cs / 2,
    /// #    },
    /// #    xoffset: Some(200),
    /// #    yoffset: Some(512),
    /// #    cache,
    /// #    mask_cache,
    /// #    mask_active: false,
    /// #    sel_start: Some(Point {x: 0., y: 0.}),
    /// #    sel_end: Some(Point {x: 256., y: 200.}),
    /// #    global_width: WIDTH,
    /// #    global_height: HEIGHT,
    /// #    cache_scale_factor_x: 2.,
    /// #    cache_scale_factor_y: 2.
    /// # };
    /// // For a selection from (0, 0) to (256, 200)
    /// let bounds = slideview.get_selection_bounds().ok_or("Couldn't get selection bounds!")?;
    /// assert_eq!(bounds.x, 0.0);
    /// assert_eq!(bounds.y, 0.0);
    /// assert_eq!((bounds.width - 24.).abs() < 0.1, true);
    /// assert_eq!((bounds.height - 73.9).abs() < 0.1, true);
    ///
    /// Ok::<(), &'static str>(())
    /// ```
    pub fn get_selection_bounds(&self) -> Option<Rectangle> {
        let vw = self.viewport_size.w;
        let vh = self.viewport_size.h;
        // TODO find out why it's +20 not +40
        let position_details = self.get_position_details();
        let hmax = position_details.hmax - 5.;
        let wmax = position_details.wmax - 5.;
        let yoffset = position_details.yoffset;
        let xoffset = position_details.xoffset;
        if let (Some(start), Some(end)) = (self.sel_start, self.sel_end) {
            let mut x0 = start.x.min(end.x) - xoffset;
            let mut y0 = start.y.min(end.y) - yoffset;
            x0 /= wmax as f32;
            y0 /= hmax as f32;
            x0 *= vw as f32;
            y0 *= vh as f32;
            let mut x1 = end.x - xoffset;
            let mut y1 = end.y - yoffset;
            x1 /= wmax as f32;
            y1 /= hmax as f32;
            x1 *= vw as f32;
            y1 *= vh as f32;
            x0 = x0.clamp(0., (vw - 1) as f32);
            y0 = y0.clamp(0., (vh - 1) as f32);
            x1 = x1.clamp(x0, vw as f32);
            y1 = y1.clamp(y0, vh as f32);
            return Some(Rectangle {
                x: x0,
                y: y0,
                width: (x1 - x0).abs(),
                height: (y1 - y0).abs(),
            });
        }
        return None;
    }
}
