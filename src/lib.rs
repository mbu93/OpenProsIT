use const_env::from_env;

pub const WIDTH: u32 = 800;
pub const HEIGHT: u32 = 600;

#[from_env]
pub const CACHE_MAX: f32 = 3000.;

pub const MOVEMENT_AMP: f32 = 2.;
pub const STEP: u32 = 4;

// Std Lib
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

// For background cache loading
use std::sync::{Arc, Mutex};

// Iced GUI
use iced::theme::Theme;

use iced::Rectangle;

// Openslide image Lib
use openslide_rs::Size as OpenslideSize;

// iced
use iced::Point;

// Local Modules
pub mod cache;
pub mod dicom_predictor;
pub mod dicom_renderer;
pub mod error;
pub mod gui_components;
pub mod image_viewer;
pub mod predictor;
pub mod pybridge;
pub mod renderer;
pub mod slide_predictor;
pub mod slide_renderer;
pub mod styles;
pub mod tracking;
pub mod util;

use cache::Border;
use error::ErrorKind;
use gui_components::Message;
use slide_predictor::SlidePredictor;
use slide_renderer::SlideView;
use tracking::Tracker;

#[derive(Copy, Clone, Debug)]
pub enum ImageType {
    DICOM,
    WSI,
}
pub struct ZoomableImageViewer {
    pub level: u32,
    pub max_level: u32,
    pub dragging: bool,
    pub drag_start: iced::Point,
    pub offsetx: f32,
    pub offsety: f32,
    pub max_extents: OpenslideSize,
    pub image_path: Vec<PathBuf>,
    pub current_image: usize,
    pub script_path: PathBuf,
    pub mouse_pos: Point,
    pub cache_scale_factor_x: f32,
    pub cache_scale_factor_y: f32,
    pub theme: Theme,
    pub mppx: Vec<f32>,
    pub mppy: Vec<f32>,
    pub info: Vec<String>,
    pub plot_data: SlideView,
    pub current_progress: usize,
    pub current_max_progress: usize,
    pub current_info: usize,
    pub update_ready: Arc<Mutex<bool>>,
    pub loadtime_offsetx: f32,
    pub loadtime_offsety: f32,
    pub loadtime_cache: Arc<Mutex<RefCell<Vec<u8>>>>,
    pub levels: Vec<f64>,
    pub current_zoom: f32,
    pub current_extents: OpenslideSize,
    pub mask_active: bool,
    pub tracker: Tracker,
    pub current_border: Border,
    pub predictor: Result<SlidePredictor, ErrorKind>,
    pub show_pred: bool,
    pub receiver: Arc<Mutex<Receiver<Message>>>,
    pub sender: Sender<Message>,
    pub draw: bool,
    pub cur_sel: Option<Rectangle>,
    pub error: Option<ErrorKind>,
    pub pred_thread_error: Arc<Mutex<Option<ErrorKind>>>,
    pub load_thread_error: Arc<Mutex<Option<ErrorKind>>>,
    pub on_border: bool,
    pub imagetype: ImageType,
}
