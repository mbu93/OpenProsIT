use crate::MOVEMENT_AMP;

#[derive(Debug)]
struct MinMaxCoords {
    minx: f32,
    miny: f32,
    maxx: f32,
    maxy: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ExtentCoords {
    pub x_right_reached: bool,
    pub y_top_reached: bool,
    pub x_left_reached: bool,
    pub y_bottom_reached: bool,
}

impl ExtentCoords {
    fn x_in_center(self) -> bool {
        return !self.x_left_reached && !self.x_right_reached;
    }
    fn y_in_center(self) -> bool {
        return !self.y_top_reached && !self.y_bottom_reached;
    }
}
#[derive(Debug)]
pub struct Limits {
    pub xcache_right_trig_reached: bool,
    pub xcache_left_trig_reached: bool,
    pub ycache_bottom_trig_reached: bool,
    pub ycache_top_trig_reached: bool,
    pub xyborder: ExtentCoords,
    pub border_reached: bool,
    pub cache_reached: bool,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Borders {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    LeftLimit,
    RightLimit,
    TopLimit,
    BottomLimit,
    TopLeftLimit,
    TopRightLimit,
    BottomLeftLimit,
    BottomRightLimit,
}

pub struct Tracker {
    pub max_global_x: f32,
    pub max_global_y: f32,
    pub min_global_x: f32,
    pub min_global_y: f32,
    pub max_cache_x: i32,
    pub max_cache_y: i32,
    pub min_cache_x: i32,
    pub min_cache_y: i32,
    pub cache_size_x: u32,
    pub cache_size_y: u32,
    pub current_x: f32,
    pub current_y: f32,
    pub center_correction_x: f32,
    pub center_correction_y: f32,
    pub preload_possible: bool,
    pub cache_scale_factor_x: f32,
    pub cache_scale_factor_y: f32,
    pub cache_comp_x: f32,
    pub cache_comp_y: f32,
}

/// A wrapper for tracking all positions and updates to be extracted for accurate rendering after
/// position updates from dragging.
///
/// Requires the following fields
///
///
/// - max_global_x: maximum x coordinates (full magnification)
/// - max_global_y: maximum y coordinates (full magnification)
/// - min_global_x: minimum x coordinates (full magnification)
/// - min_global_y: minimum y coordinates (full magnification)
/// - max_cache_x: pos cache limit for x
/// - max_cache_y: pos cache limit for y
/// - min_cache_x: neg cache limit for x
/// - min_cache_y: neg cache limit for y
/// - cache_size_x: cache width
/// - cache_size_y: cache heigh
/// - current_x: current x position (full magnification)
/// - current_y: current y position (full magnification)
/// - center_correction_x: x correction imposed by switching drag directions (px),
/// - center_correction_y: y correction imposed by switching drag directions (px),
/// - preload_possible: bool,
/// - cache_scale_factor_x: relation between x cache size / viewport size,
/// - cache_scale_factor_y: relation between y cache size / viewport size,
/// - cache_comp: factor to correct from dividing level / available downsample
impl Tracker {
    /// Update the currently stored coordinates.
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::{tracking::Tracker};
    ///
    /// let mut tracker = Tracker {
    ///         max_global_x: 2048.,
    ///         min_global_x: 0.,
    ///         max_global_y: 2048.,
    ///         min_global_y: 0.,
    ///         max_cache_x: 256,
    ///         min_cache_x: -256,
    ///         max_cache_y: 256,
    ///         min_cache_y: -256,
    ///         cache_size_x: 512,
    ///         cache_size_y: 512,
    ///         current_x: 512.,
    ///         current_y: 512.,
    ///         center_correction_x: 0.,
    ///         center_correction_y: 0.,
    ///         preload_possible: true,
    ///         cache_scale_factor_x: 2.,
    ///         cache_scale_factor_y: 2.,
    ///         cache_comp_x: 1.,
    ///         cache_comp_y: 1.,
    /// };
    /// // Note: Delta is amplified by MOVEMENT_AMP (2).
    /// // No clipping is applied. Global coords are kept, cache is updated.
    /// let mut global_x = 512.;
    /// let mut global_y = 512.;
    /// let mut cache_x = 0.;
    /// let mut cache_y = 0.;
    /// let limits = tracker.update_coords(1, 1, &mut global_x, &mut global_y, &mut cache_x, &mut cache_y, 10., -10.);
    /// assert_eq!(global_x, 512.);
    /// assert_eq!(global_y, 512.);
    /// assert_eq!(cache_x, -20.);
    /// assert_eq!(cache_y, 20.);
    ///
    /// assert_eq!(limits.xcache_right_trig_reached, false);
    /// assert_eq!(limits.xcache_left_trig_reached, false);
    /// assert_eq!(limits.ycache_bottom_trig_reached, false);
    /// assert_eq!(limits.ycache_top_trig_reached, false);
    /// assert_eq!(limits.border_reached, false);
    /// assert_eq!(limits.cache_reached, false);
    ///
    /// // No clipping is applied. Global coords are update, cache is updated.
    /// let mut global_x = 512.;
    /// let mut global_y = 512.;
    /// tracker.current_x = 512.;
    /// tracker.current_y = 512.;
    /// let mut cache_x = 0.;
    /// let mut cache_y = 0.;
    /// let limits = tracker.update_coords(1, 1, &mut global_x, &mut global_y, &mut cache_x, &mut cache_y, 50., -50.);
    /// assert_eq!(global_x, 384.);
    /// assert_eq!(global_y, 640.);
    /// assert_eq!(cache_x, -100.);
    /// assert_eq!(cache_y, 100.);
    ///
    /// assert_eq!(limits.xcache_right_trig_reached, false);
    /// assert_eq!(limits.xcache_left_trig_reached, true);
    /// assert_eq!(limits.ycache_bottom_trig_reached, true);
    /// assert_eq!(limits.ycache_top_trig_reached, false);
    /// assert_eq!(limits.border_reached, false);
    /// assert_eq!(limits.cache_reached, true);
    ///
    /// // Switching directions after cache update still provides correct variables
    /// let mut global_x = 512.;
    /// let mut global_y = 512.;
    /// tracker.current_x = 512.;
    /// tracker.current_y = 512.;
    /// let mut cache_x = 0.;
    /// let mut cache_y = 0.;
    /// let limits = tracker.update_coords(1, 1, &mut global_x, &mut global_y, &mut cache_x, &mut cache_y, -63., 63.);
    /// let limits = tracker.update_coords(1, 1, &mut global_x, &mut global_y, &mut cache_x, &mut cache_y, 126., -126.);
    ///
    /// assert_eq!(global_x, 384.);
    /// assert_eq!(global_y, 640.);
    /// assert_eq!(cache_x, -126.);
    /// assert_eq!(cache_y, 126.);
    ///
    /// assert_eq!(limits.xcache_right_trig_reached, false);
    /// assert_eq!(limits.xcache_left_trig_reached, true);
    /// assert_eq!(limits.ycache_bottom_trig_reached, true);
    /// assert_eq!(limits.ycache_top_trig_reached, false);
    /// assert_eq!(limits.border_reached, false);
    /// assert_eq!(limits.cache_reached, true);
    ///
    /// // Coord clipping is applied.
    /// let mut global_x = 512.;
    /// let mut global_y = 2047.;
    /// tracker.current_x = 512.;
    /// tracker.current_y = 2047.;
    /// let mut cache_x = 0.;
    /// let mut cache_y = 0.;
    /// let limits = tracker.update_coords(1, 1, &mut global_x, &mut global_y, &mut cache_x, &mut cache_y, 0., -63.);
    /// assert_eq!(global_x, 512.);
    /// assert_eq!(global_y, 2048.);
    /// assert_eq!(cache_x, 0.);
    /// assert_eq!(cache_y, 126.);
    /// let mut global_x = 0.;
    /// let mut global_y = 2047.;
    /// tracker.current_x = 0.;
    /// tracker.current_y = 2047.;
    /// let mut cache_x = 0.;
    /// let mut cache_y = 0.;
    /// let limits = tracker.update_coords(1, 1, &mut global_x, &mut global_y, &mut cache_x, &mut cache_y, 63., 0.);
    /// assert_eq!(global_x, 0.);
    /// assert_eq!(global_y, 2047.);
    /// assert_eq!(cache_x, -126.);
    /// assert_eq!(cache_y, 0.);
    ///
    /// // Cache clipping is applied.
    /// let mut global_x = 512.;
    /// let mut global_y = 512.;
    /// let mut cache_x = 0.;
    /// let mut cache_y = 0.;
    /// tracker.current_x = 512.;
    /// tracker.current_y = 512.;
    /// let limits = tracker.update_coords(1, 1, &mut global_x, &mut global_y, &mut cache_x, &mut cache_y, 129., -129.);
    /// assert_eq!(global_x, 384.);
    /// assert_eq!(global_y, 640.);
    /// assert_eq!(cache_x, 0.);
    /// assert_eq!(cache_y, 0.);
    /// assert_eq!(limits.border_reached, true);
    /// ```
    pub fn update_coords(
        &mut self,
        level: u32,
        original_level: u32,
        global_x: &mut f32,
        global_y: &mut f32,
        cache_x: &mut f32,
        cache_y: &mut f32,
        delta_x: f32,
        delta_y: f32,
    ) -> Limits {
        let mut limits = Limits {
            xcache_right_trig_reached: false,
            xcache_left_trig_reached: false,
            ycache_bottom_trig_reached: false,
            ycache_top_trig_reached: false,
            border_reached: false,
            cache_reached: false,
            xyborder: ExtentCoords::default(),
        };
        *cache_x -= delta_x * MOVEMENT_AMP;
        *cache_y -= delta_y * MOVEMENT_AMP;
        let viewport_size_x = self.cache_size_x as f32 / self.cache_scale_factor_x;
        let viewport_size_y = self.cache_size_y as f32 / self.cache_scale_factor_y;
        let mut x_right_reached =
            *cache_x >= (self.cache_size_x as f32 / 2. - viewport_size_x / 2.) / 2.;
        let mut y_bottom_reached =
            *cache_y >= (self.cache_size_y as f32 / 2. - viewport_size_y / 2.) / 2.;
        let mut x_left_reached =
            *cache_x <= ((-1. * self.cache_size_x as f32 / 2.) + viewport_size_x / 2.) / 2.;
        let mut y_top_reached =
            *cache_y <= ((-1. * self.cache_size_y as f32 / 2.) + viewport_size_y / 2.) / 2.;
        limits.xcache_right_trig_reached = x_right_reached;
        limits.xcache_left_trig_reached = x_left_reached;
        limits.ycache_bottom_trig_reached = y_bottom_reached;
        limits.ycache_top_trig_reached = y_top_reached;
        let xyborder = self.check_coords(global_x, global_y, original_level);
        limits.xyborder = xyborder;
        let border = self.get_current_border(&limits);
        self.set_global_coords(global_x, global_y, level, &border);
        if x_right_reached || y_bottom_reached || x_left_reached || y_top_reached {
            limits.cache_reached = true;
        }
        x_right_reached = *cache_x >= self.cache_size_x as f32 / 2. - viewport_size_x / 2.;
        y_bottom_reached = *cache_y >= self.cache_size_y as f32 / 2. - viewport_size_y / 2.;
        x_left_reached = *cache_x <= (-1. * self.cache_size_x as f32 / 2.) + viewport_size_x / 2.;
        y_top_reached = *cache_y <= (-1. * self.cache_size_y as f32 / 2.) + viewport_size_y / 2.;

        if ((x_right_reached & !limits.xyborder.x_right_reached)
            || (y_bottom_reached & !limits.xyborder.y_bottom_reached)
            || (x_left_reached & !limits.xyborder.x_left_reached)
            || (y_top_reached & !limits.xyborder.y_top_reached))
            & self.preload_possible
        {
            limits.border_reached = true;

            let limits_ = Limits {
                xcache_right_trig_reached: x_right_reached,
                xcache_left_trig_reached: x_left_reached,
                ycache_bottom_trig_reached: y_bottom_reached,
                ycache_top_trig_reached: y_top_reached,
                border_reached: true,
                cache_reached: false,
                xyborder: limits.xyborder,
            };
            self.clip_global_coords(global_x, global_y, level);
            self.clip_cache_coords(cache_x, cache_y);

            self.current_x = *global_x;
            self.current_y = *global_y;
            self.center_correction_x = 0.;
            self.center_correction_y = 0.;
            self.compensate_offsets(cache_x, cache_y, limits_, &border);
        }
        self.clip_global_coords(global_x, global_y, original_level);
        self.clip_cache_coords(cache_x, cache_y);
        return limits;
    }

    /// Clip the provided cache coordinates to not exceed the current cache size.
    ///  
    /// Example
    /// ```
    /// # use slideslib::{tracking::Tracker};
    ///
    /// let mut tracker = Tracker {
    ///         max_global_x: 2048.,
    ///         min_global_x: 0.,
    ///         max_global_y: 2048.,
    ///         min_global_y: 0.,
    ///         max_cache_x: 256,
    ///         min_cache_x: -256,
    ///         max_cache_y: 256,
    ///         min_cache_y: -256,
    ///         cache_size_x: 512,
    ///         cache_size_y: 512,
    ///         current_x: 512.,
    ///         current_y: 512.,
    ///         center_correction_x: 0.,
    ///         center_correction_y: 0.,
    ///         preload_possible: false,
    ///         cache_scale_factor_x: 2.,
    ///         cache_scale_factor_y: 2.,
    ///         cache_comp_x: 1.,
    ///         cache_comp_y: 1.,
    /// };
    /// let mut cache_x = 270.;
    /// let mut cache_y = -270.;
    /// tracker.clip_cache_coords(&mut cache_x, &mut cache_y);
    /// assert_eq!(cache_x, 128.);
    /// assert_eq!(cache_y, -128.);
    /// ```
    pub fn clip_cache_coords(&self, cache_x: &mut f32, cache_y: &mut f32) {
        let sfx = self.cache_scale_factor_x;
        let sfy = self.cache_scale_factor_y;
        *cache_y = (*cache_y).clamp(
            self.cache_size_y as f32 / (2. * sfy) - self.cache_size_y as f32 / 2.,
            self.cache_size_y as f32 / 2. - self.cache_size_y as f32 / (2. * sfy),
        );
        *cache_x = (*cache_x).clamp(
            self.cache_size_x as f32 / (2. * sfx) - self.cache_size_x as f32 / 2.,
            self.cache_size_x as f32 / 2. - self.cache_size_x as f32 / (2. * sfx),
        );
    }
    /// Calculates the current available minimum x and y positions including a buffer for
    /// borders.
    fn get_min_max(&self, original_level: u32) -> MinMaxCoords {
        let width = self.max_global_x / original_level as f32;
        let height = self.max_global_y / original_level as f32;
        let maxx = width * original_level as f32;
        let miny = 0.;
        let minx = 0.;
        let maxy = height * original_level as f32;
        return MinMaxCoords {
            minx,
            maxx,
            miny,
            maxy,
        };
    }
    /// Clip the provided global coordinates to not exceed the current slide size.
    ///  
    /// Example
    /// ```
    /// # use slideslib::{tracking::Tracker};
    ///
    /// let mut tracker = Tracker {
    ///         max_global_x: 2048.,
    ///         min_global_x: 0.,
    ///         max_global_y: 2048.,
    ///         min_global_y: 0.,
    ///         max_cache_x: 256,
    ///         min_cache_x: -256,
    ///         max_cache_y: 256,
    ///         min_cache_y: -256,
    ///         cache_size_x: 512,
    ///         cache_size_y: 512,
    ///         current_x: 512.,
    ///         current_y: 512.,
    ///         center_correction_x: 0.,
    ///         center_correction_y: 0.,
    ///         preload_possible: false,
    ///         cache_scale_factor_x: 2.,
    ///         cache_scale_factor_y: 2.,
    ///         cache_comp_x: 1.,
    ///         cache_comp_y: 1.,
    /// };
    /// let mut cache_x = 0.;
    /// let mut cache_y = 2048.;
    ///
    /// tracker.clip_global_coords(&mut cache_x, &mut cache_y, 1);
    /// assert_eq!(cache_x, 0.);
    /// assert_eq!(cache_y, 2048.);
    /// ```
    pub fn clip_global_coords(
        &mut self,
        global_x: &mut f32,
        global_y: &mut f32,
        original_level: u32,
    ) {
        let coords = self.get_min_max(original_level);
        *global_y = (*global_y).clamp(coords.miny, coords.maxy);
        *global_x = (*global_x).clamp(coords.minx, coords.maxx);
    }

    fn check_coords(
        &mut self,
        global_x: &mut f32,
        global_y: &mut f32,
        original_level: u32,
    ) -> ExtentCoords {
        let coords = self.get_min_max(original_level);
        let mut extent_coords = ExtentCoords::default();
        if *global_y <= coords.miny {
            extent_coords.y_top_reached = true;
        }
        if *global_y >= coords.maxy {
            extent_coords.y_bottom_reached = true;
        }
        if *global_x <= coords.minx {
            extent_coords.x_left_reached = true;
        }
        if *global_x >= coords.maxx {
            extent_coords.x_right_reached = true;
        }
        return extent_coords;
    }

    fn compensate_offsets(
        &mut self,
        cache_x: &mut f32,
        cache_y: &mut f32,
        limits: Limits,
        border: &Borders,
    ) {
        let sfy = self.cache_scale_factor_y;
        let sfx = self.cache_scale_factor_x;
        let miny = -1. * self.cache_size_y as f32 / (2. * sfy);
        let maxy = self.cache_size_y as f32 / (2. * sfy);
        let minx = -1. * self.cache_size_x as f32 / (2. * sfx);
        let maxx = self.cache_size_x as f32 / (2. * sfx);
        let viewport_size_x = self.cache_size_x as f32 / self.cache_scale_factor_x;
        let viewport_size_y = self.cache_size_y as f32 / self.cache_scale_factor_y;
        // X neg -> Left
        // X pos -> Right
        // Y neg -> Top
        // Y pos -> Bot
        match border {
            Borders::Top => {
                *cache_y = 0.;
            }
            Borders::Bottom => {
                *cache_y = 0.;
            }
            Borders::Right => {
                *cache_x = 0.;
            }
            Borders::Left => {
                *cache_x = 0.;
            }
            Borders::TopLeft => {
                if limits.xcache_left_trig_reached & limits.xyborder.x_in_center() {
                    *cache_x = 0.;
                    let mut dist = self.cache_size_y as f32 / 2. - viewport_size_y / 2.;
                    dist -= cache_y.abs();
                    if limits.xyborder.y_in_center() {
                        *cache_y = dist;
                    } else {
                        *cache_y = miny + dist;
                    }
                    return;
                }
                if limits.ycache_top_trig_reached & limits.xyborder.y_in_center() {
                    *cache_y = 0.;
                    let mut dist = self.cache_size_x as f32 / 2. - viewport_size_x / 2.;
                    dist -= cache_x.abs();
                    if limits.xyborder.x_in_center() {
                        *cache_x = dist
                    } else {
                        *cache_x = minx + dist;
                    }
                }
            }
            Borders::BottomLeft => {
                if limits.xcache_left_trig_reached & limits.xyborder.x_in_center() {
                    *cache_x = 0.;
                    let mut dist = self.cache_size_y as f32 / 2. - viewport_size_y / 2.;
                    dist -= cache_y.abs();
                    if limits.xyborder.y_in_center() {
                        *cache_y = -dist;
                    } else {
                        *cache_y = maxy - dist;
                    }
                    return;
                }

                if limits.ycache_bottom_trig_reached & limits.xyborder.y_in_center() {
                    *cache_y = 0.;
                    let mut dist = self.cache_size_x as f32 / 2. - viewport_size_x / 2.;
                    dist -= cache_x.abs();
                    if limits.xyborder.x_in_center() {
                        *cache_x = dist
                    } else {
                        *cache_x = minx + dist;
                    }
                }
            }
            Borders::TopRight => {
                if limits.xcache_right_trig_reached & limits.xyborder.x_in_center() {
                    *cache_x = 0.;
                    let mut dist = self.cache_size_y as f32 / 2. - viewport_size_y / 2.;
                    dist -= cache_y.abs();
                    if limits.xyborder.y_in_center() {
                        *cache_y = dist;
                    } else {
                        *cache_y = miny + dist;
                    }
                }
                if limits.ycache_top_trig_reached & limits.xyborder.y_in_center() {
                    *cache_y = 0.;
                    let mut dist = self.cache_size_x as f32 / 2. - viewport_size_x / 2.;
                    dist -= cache_x.abs();
                    if limits.xyborder.x_in_center() {
                        *cache_x = -dist
                    } else {
                        *cache_x = maxx - dist;
                    }
                }
            }
            Borders::BottomRight => {
                if limits.xcache_right_trig_reached & limits.xyborder.x_in_center() {
                    *cache_x = 0.;
                    let mut dist = self.cache_size_y as f32 / 2. - viewport_size_y / 2.;
                    dist -= cache_y.abs();
                    if limits.xyborder.y_in_center() {
                        *cache_y = -dist;
                    } else {
                        *cache_y = maxy - dist;
                    }
                }

                if limits.ycache_bottom_trig_reached & limits.xyborder.y_in_center() {
                    *cache_y = 0.;
                    let mut dist = self.cache_size_x as f32 / 2. - viewport_size_x / 2.;
                    dist -= cache_x.abs();
                    if limits.xyborder.x_in_center() {
                        *cache_x = -dist
                    } else {
                        *cache_x = maxx - dist;
                    }
                }
            }
            _ => println!("position unchanged"),
        }
    }

    /// Get the current border according to the limits calculated by the tracker instance.
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::tracking::{Tracker, Limits, Borders, ExtentCoords};
    ///
    /// let mut tracker = Tracker {
    ///         max_global_x: 2048.,
    ///         min_global_x: 0.,
    ///         max_global_y: 2048.,
    ///         min_global_y: 0.,
    ///         max_cache_x: 256,
    ///         min_cache_x: -256,
    ///         max_cache_y: 256,
    ///         min_cache_y: -256,
    ///         cache_size_x: 512,
    ///         cache_size_y: 512,
    ///         current_x: 512.,
    ///         current_y: 512.,
    ///         center_correction_x: 0.,
    ///         center_correction_y: 0.,
    ///         preload_possible: false,
    ///         cache_scale_factor_x: 1.,
    ///         cache_scale_factor_y: 1.,
    ///         cache_comp_x: 1.,
    ///         cache_comp_y: 1.,
    /// };
    /// let mut cache_x = 270.;
    /// let mut cache_y = -270.;
    /// let limits = Limits {
    ///     xcache_right_trig_reached: false,
    ///     xcache_left_trig_reached: false,
    ///     ycache_bottom_trig_reached: false,
    ///     ycache_top_trig_reached: true,
    ///     xyborder: ExtentCoords {  
    ///         x_right_reached: false,
    ///         y_top_reached: false,
    ///         x_left_reached: false,
    ///         y_bottom_reached: false,
    ///     },
    ///     border_reached: false,
    ///     cache_reached: true,
    /// };
    /// let border = tracker.get_current_border(&limits);
    /// assert_eq!(border, Borders::Top);
    /// ```
    pub fn get_current_border(&self, limits: &Limits) -> Borders {
        if limits.xcache_right_trig_reached & limits.ycache_top_trig_reached {
            if limits.xyborder.y_top_reached & (limits.xyborder.x_right_reached) {
                return Borders::TopRightLimit;
            }
            if !limits.xyborder.y_top_reached & limits.xyborder.x_right_reached {
                return Borders::Top;
            }
            if limits.xyborder.y_top_reached & !limits.xyborder.x_right_reached {
                return Borders::Right;
            }
            return Borders::TopRight;
        } else if limits.xcache_right_trig_reached & limits.ycache_bottom_trig_reached {
            if limits.xyborder.y_bottom_reached & limits.xyborder.x_right_reached {
                return Borders::BottomRightLimit;
            }
            if !limits.xyborder.y_bottom_reached & limits.xyborder.x_right_reached {
                return Borders::Bottom;
            }
            if limits.xyborder.y_bottom_reached & !limits.xyborder.x_right_reached {
                return Borders::Right;
            }
            return Borders::BottomRight;
        } else if limits.xcache_left_trig_reached & limits.ycache_top_trig_reached {
            if limits.xyborder.y_top_reached & limits.xyborder.x_left_reached {
                return Borders::TopLeftLimit;
            }
            if !limits.xyborder.y_top_reached & limits.xyborder.x_left_reached {
                return Borders::Top;
            }
            if limits.xyborder.y_top_reached & !limits.xyborder.x_left_reached {
                return Borders::Left;
            }
            return Borders::TopLeft;
        } else if limits.xcache_left_trig_reached & limits.ycache_bottom_trig_reached {
            if limits.xyborder.y_bottom_reached & limits.xyborder.x_left_reached {
                return Borders::BottomLeftLimit;
            }
            if !limits.xyborder.y_bottom_reached & limits.xyborder.x_left_reached {
                return Borders::Bottom;
            }
            if limits.xyborder.y_bottom_reached & !limits.xyborder.x_left_reached {
                return Borders::Left;
            }
            return Borders::BottomLeft;
        } else if limits.xcache_right_trig_reached {
            if limits.xyborder.x_right_reached {
                if limits.ycache_bottom_trig_reached {
                    return Borders::BottomRight;
                }
                if limits.ycache_top_trig_reached {
                    return Borders::TopRight;
                }
                return Borders::RightLimit;
            }
            return Borders::Right;
        } else if limits.xcache_left_trig_reached {
            if limits.xyborder.x_left_reached {
                if limits.ycache_bottom_trig_reached {
                    return Borders::BottomLeft;
                }
                if limits.ycache_top_trig_reached {
                    return Borders::TopLeft;
                }

                return Borders::LeftLimit;
            }
            return Borders::Left;
        } else if limits.ycache_top_trig_reached {
            if limits.xyborder.y_top_reached {
                if limits.xcache_right_trig_reached {
                    return Borders::TopRight;
                }
                if limits.xcache_left_trig_reached {
                    return Borders::TopLeft;
                }
                return Borders::TopLimit;
            }
            return Borders::Top;
        } else if limits.ycache_bottom_trig_reached {
            if limits.xyborder.y_bottom_reached {
                if limits.xcache_right_trig_reached {
                    return Borders::BottomRight;
                }
                if limits.xcache_left_trig_reached {
                    return Borders::BottomLeft;
                }
                return Borders::BottomLimit;
            }
            return Borders::Bottom;
        } else {
            return Borders::Center;
        }
    }

    fn set_global_coords(&mut self, x: &mut f32, y: &mut f32, level: u32, border: &Borders) {
        let mut correction_x = 0.;
        let mut correction_y = 0.;
        let viewport_size_x = self.cache_size_x as f32 / self.cache_scale_factor_x;
        let viewport_size_y = self.cache_size_y as f32 / self.cache_scale_factor_y;
        match border {
            Borders::Top => {
                correction_y = (-1. * self.cache_size_y as f32 / 2. + viewport_size_y / 2.)
                    / self.cache_comp_y
                    * level as f32;
                *y = self.current_y + correction_y;
                *x = self.current_x;
            }
            Borders::Bottom => {
                correction_y = (self.cache_size_y as f32 / 2. - viewport_size_y / 2.)
                    / self.cache_comp_y
                    * level as f32;
                *y = self.current_y + correction_y;
                *x = self.current_x;
            }
            Borders::Right => {
                correction_x = (self.cache_size_x as f32 / 2. - viewport_size_x / 2.)
                    / self.cache_comp_x
                    * level as f32;
                *x = self.current_x + correction_x;
                *y = self.current_y;
            }
            Borders::Left => {
                correction_x = (-1. * self.cache_size_x as f32 / 2. + viewport_size_x / 2.)
                    / self.cache_comp_x
                    * level as f32;
                *x = self.current_x + correction_x;
                *y = self.current_y;
            }
            Borders::Center => {
                *x = self.current_x - self.center_correction_x;
                *y = self.current_y - self.center_correction_y;
            }
            Borders::TopLeft => {
                correction_x = (-1. * self.cache_size_x as f32 / 2. + viewport_size_x / 2.)
                    / self.cache_comp_x
                    * level as f32;
                correction_y = (-1. * self.cache_size_y as f32 / 2. + viewport_size_y / 2.)
                    / self.cache_comp_y
                    * level as f32;
                *x = self.current_x + correction_x;
                *y = self.current_y + correction_y;
            }
            Borders::TopRight => {
                correction_x = (self.cache_size_x as f32 / 2. - viewport_size_x / 2.)
                    / self.cache_comp_x
                    * level as f32;
                correction_y = (-1. * self.cache_size_y as f32 / 2. + viewport_size_y / 2.)
                    / self.cache_comp_y
                    * level as f32;
                *x = self.current_x + correction_x;
                *y = self.current_y + correction_y;
            }
            Borders::BottomLeft => {
                correction_x = (-1. * self.cache_size_x as f32 / 2. + viewport_size_x / 2.)
                    / self.cache_comp_x
                    * level as f32;
                correction_y = (self.cache_size_y as f32 / 2. - viewport_size_y / 2.)
                    / self.cache_comp_y
                    * level as f32;
                *x = self.current_x + correction_x;
                *y = self.current_y + correction_y;
            }
            Borders::BottomRight => {
                correction_x = (self.cache_size_x as f32 / 2. - viewport_size_x / 2.)
                    / self.cache_comp_x
                    * level as f32;
                correction_y = (self.cache_size_y as f32 / 2. - viewport_size_y / 2.)
                    / self.cache_comp_y
                    * level as f32;
                *x = self.current_x + correction_x;
                *y = self.current_y + correction_y;
            }
            _ => {}
        }
        self.center_correction_x = correction_x;
        self.center_correction_y = correction_y;
    }
}
