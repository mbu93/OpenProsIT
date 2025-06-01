// STD lib
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::ffi::OsStr;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::{fs, thread, time::Duration};

// For background cache loading
use std::sync::{Arc, Mutex};

// NDArray
use ndarray::{s, Array, ArrayView};

// Iced GUI
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Location};
use iced::subscription::Subscription;
use iced::theme::Theme;
use iced::widget::container::StyleSheet;
use iced::widget::{progress_bar, scrollable};
use iced::widget::Button;
use iced::widget::Text;
use iced::widget::{row, Column, Container};
use iced::{event, keyboard, mouse, window};
use iced::{executor, theme, Application, Color, Command, Element, Event, Length};

// Iced additional widgets
use iced_aw::{split, Split};

// Openslide image Lib
use openslide_rs::Size as OpenslideSize;

// FileDialog
use rfd::FileDialog;

// OpenSlide
use openslide_rs::traits::Slide;
use openslide_rs::OpenSlide;

// Pyo3
use pyo3::prepare_freethreaded_python;

// Glob
use glob::glob;

// Crate modules
use crate::cache::{
    change_cache, find_next_greater_value, reset_offsets, update_cache_data, update_offsets,
    update_zoom_props, Border,
};
use crate::dicom_predictor::DicomPredictor;
use crate::dicom_renderer::DicomView;
use crate::error::ErrorKind;
use crate::gui_components::{
    default_menu, labeled_button, labeled_list_button, modal, Message, Modal,
};
use crate::predictor::{Predictor, PredictorArgs};
use crate::pybridge::execute_script_for_file;
use crate::renderer::{get_viewport_bounds, BaseViewArgs};
use crate::slide_predictor::{replace_suffix_with_pred, CounterUpdateSubscription, SlidePredictor};
use crate::slide_renderer::SlideView;
use crate::styles::{ProgressStyle, TopbarStyle};
use crate::tracking::{Borders, Limits, Tracker};
use crate::util::{get_file_list, log_or_load_thread_err, reset_thread_err};
use crate::ImageType;
use crate::STEP;
use crate::{ZoomableImageViewer, CACHE_MAX};
pub const NOINFOTEXT: &str = "No info available yet!";


fn wait_until_file_ready(path: &str, max_wait_secs: u64) -> std::io::Result<()> {
    let mut last_size = 0;
    for _ in 0..max_wait_secs * 1000 {
        if let Ok(metadata) = fs::metadata(path) {
            let current_size = metadata.len();
            if current_size > 0 && current_size == last_size {
                return Ok(());
            }
            last_size = current_size;
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "File did not stabilize"))
}

fn get_path(filtername: &str, filters: &[&str], start_path: &str, single: bool) -> PathBuf {
    let file_dialog = FileDialog::new()
        .add_filter(filtername, &filters)
        .set_directory(start_path);
    let path = match single {
        true => file_dialog.pick_file(),
        _ => file_dialog.pick_folder(),
    };
    match path {
        Some(val) => val,
        _ => {
            println!("Error selecting file.");
            PathBuf::from("")
        }
    }
}

fn change_file(data: &mut ZoomableImageViewer, index: usize) -> Result<(), ErrorKind> {
    match data.imagetype {
        ImageType::WSI => {
            data.current_image = index;
            data.current_info = index;
            reset_offsets(data);
            load_slide(data, None)?
        }
        _ => load_dicom(data, None)?,
    };
    Ok(())
}

/// Load a slide according to the currently specified path. Invalid paths or properties will cause
/// an error.
///
/// Example:
///
/// ```
/// # use slideslib::{ZoomableImageViewer, image_viewer::load_slide};
/// # use slideslib::error::ErrorKind;
/// # use iced::application::Application;
/// # use std::path::PathBuf;
/// # use std::vec::Vec;
///
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.current_image = 0;
/// viewer.image_path = Vec::from([PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff")]);
/// viewer.level = 16;
/// let slide = load_slide(&mut viewer, Some(16))?;
/// assert_eq!(viewer.max_level, 16);
/// assert_eq!(viewer.level, 16);
/// assert_eq!(viewer.offsetx, 11008.);
/// assert_eq!(viewer.offsety, 2304.);
///
/// // Non existent slide throws a handled error;
/// viewer.image_path = Vec::from([PathBuf::from("tests/data/none.tiff")]);
/// let slide = load_slide(&mut viewer, Some(16)).unwrap_err();
/// assert!(matches!(slide, ErrorKind), "Error was not detected when loading invalid slide!");
///
/// // Existing but broken slide throws a handled error;
/// viewer.image_path = Vec::from([PathBuf::from("tests/data/mock.svs")]);
/// let slide = load_slide(&mut viewer, Some(16)).unwrap_err();
/// assert!(matches!(slide, ErrorKind), "Error was not detected when loading invalid slide!");
///
/// Ok::<(), ErrorKind>(())
/// ```
pub fn load_slide(data: &mut ZoomableImageViewer, level: Option<u32>) -> Result<u8, ErrorKind> {
    let data_path = &data
        .image_path
        .get(data.current_image)
        .ok_or(())
        .map_err(|_| ErrorKind::NoFileError())?;
    if let Ok(slide) = OpenSlide::new(data_path) {
        if let Ok(levels) = slide.get_all_level_downsample() {
            data.levels = levels.clone();
            if let Some(max_level) = levels.last() {
                data.max_level = max_level.clone() as u32;
                data.level = level.unwrap_or(data.max_level);
                let dims = match slide.get_level_dimensions(0) {
                    Ok(val) => val,
                    Err(_) => {
                        return Err(ErrorKind::OpenSlidePropertiesError(
                            data.image_path[data.current_image].clone(),
                        )
                        .into())
                    }
                };

                data.offsetx = dims.w as f32 / 2.;
                data.offsety = dims.h as f32 / 2.;
                data.max_extents = dims;
                update_zoom_props(data);
                update_cache_data(data, false, data.imagetype.clone());
            } else {
                return Err(ErrorKind::OpenSlidePropertiesError(
                    data.image_path[data.current_image].clone(),
                )
                .into());
            }
        } else {
            return Err(ErrorKind::OpenSlidePropertiesError(
                data.image_path[data.current_image].clone(),
            )
            .into());
        }
    } else {
        data.info.remove(data.current_info);
        data.image_path.remove(data.current_image);
        data.current_image = 0;
        data.current_info = 0;
        if data.info.len() < 1 || data.image_path.len() < 1 {
            data.info = Vec::from([String::from(NOINFOTEXT)]);
            data.image_path = Vec::from([PathBuf::new()]);
        } else {
            load_slide(data, None)?;
        }
        return Err(ErrorKind::OpenSlideImageLoadingError(
            data.image_path[data.current_image].clone(),
        )
        .into());
    }
    match data.predictor {
        Ok(ref mut predictor) => {
            predictor.image_path = data.image_path[data.current_image].clone();
        }
        _ => {}
    }
    data.show_pred = false;
    return Ok(0);
}

/// Load a dicom according to the currently specified path. Invalid paths or properties will cause
/// an error.
///
/// Example:
///
/// ```
/// # use slideslib::{ZoomableImageViewer, image_viewer::{load_dicom, find_parent_of_mpmri}};
/// # use slideslib::error::ErrorKind;
/// # use iced::application::Application;
/// # use std::path::PathBuf;
/// # use std::vec::Vec;
///
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.current_image = 0;
/// viewer.image_path = Vec::from([PathBuf::from("tests/MRI Test")]);
/// viewer.level = 5;
///
/// // Folder was selected
/// viewer.image_path = Vec::from([PathBuf::from("tests/MRI Test")]);
/// let dcm = load_dicom(&mut viewer, Some(5));
/// assert!(matches!(dcm, Ok(_)));
/// let cache = &viewer.plot_data.view.cache.borrow();
/// let sum: u32 = cache[224*50*3..224*50*3+150].iter().map(|x| *x as u32).sum();
/// assert!(sum > 0);
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.current_image = 0;
/// viewer.image_path = Vec::from([PathBuf::from("tests/MRI Test")]);
/// viewer.level = 10;
/// // Non existent dicom throws a handled error;
///
/// viewer.image_path = Vec::from([PathBuf::from("tests/data/none.tiff")]);
/// let dcm = load_dicom(&mut viewer, Some(10)).unwrap_err();
/// assert!(matches!(dcm, ErrorKind), "Error was not detected when loading invalid dcm!");
///
/// let mut viewer = ZoomableImageViewer::new(()).0;
/// viewer.current_image = 0;
/// viewer.image_path = Vec::from([PathBuf::from("tests/MRI Test")]);
/// viewer.level = 10;
///
/// // File was selected rather than folder
/// viewer.image_path = Vec::from([PathBuf::from("tests/MRI Test/mpMRI/t2w/slice_000.dcm")]);
/// # let path = viewer.image_path[viewer.current_image].clone();
/// # match find_parent_of_mpmri(path.clone()) {
/// #     Some(subpath) => {
/// #         viewer.image_path = Vec::new();
/// #         viewer.image_path.push(subpath);
/// #         viewer.current_image = 0;
/// #         viewer.current_info = 0;
/// #         viewer.current_progress = 0;
/// #     }
/// #     None => viewer.error = Some(ErrorKind::DicomImageLoadingError(path)),
/// # };
/// // Note that when using the app, the file would be selected and thus the filename is processed
/// // inbetween.
/// let dcm = load_dicom(&mut viewer, Some(0));
/// assert!(matches!(dcm, Ok(_)));
/// let sum: u32 = cache[224*50*3..224*50*3+150].iter().map(|x| *x as u32).sum();
/// assert!(sum > 0);
///
/// Ok::<(), ErrorKind>(())
/// ```
pub fn load_dicom(data: &mut ZoomableImageViewer, _level: Option<u32>) -> Result<u8, ErrorKind> {
    let arr: Vec<u8> = Vec::new();
    // preprocess data
    match execute_script_for_file(
        data,
        &arr,
        0,
        0,
        String::from("mri_extractor"),
        String::from("pyfunctions"),
        String::from(data.image_path[data.current_image].to_str().unwrap_or("")),
    ) {
        Ok((info, _)) => {
            let path = &data.image_path[data.current_image];
            data.info[0] = info;
            data.image_path[data.current_image] = Path::new("data").join("preprocessed").join(
                path.file_name()
                    .unwrap_or(OsStr::new(""))
                    .to_str()
                    .unwrap_or(""),
            );
        }

        Err(err) => {
            return Err(ErrorKind::ScriptError(
                String::from(data.image_path[data.current_image].to_str().unwrap_or("")),
                String::from("mri_extractor.py"),
                err.to_string(),
            ));
        }
    };
    data.error = update_cache_data(data, false, ImageType::DICOM);

    // Cache and viewport are equal in this case
    data.cache_scale_factor_x = 1.;
    data.cache_scale_factor_y = 1.;
    data.plot_data.view.cache_scale_factor_x = 1.;
    data.plot_data.view.cache_scale_factor_y = 1.;

    // Set the sizes according to the preprocessing (224, 224*3, D)
    data.plot_data.view.cache_size.w = 224 * 3;
    data.plot_data.view.cache_size.h = 224;
    data.plot_data.view.viewport_size.w = 224 * 3;
    data.plot_data.view.viewport_size.h = 224;
    Ok(0)
}

pub fn load_data(data: &mut ZoomableImageViewer, level: Option<u32>) -> Result<u8, ErrorKind> {
    match data.imagetype {
        ImageType::WSI => load_slide(data, level),
        _ => load_dicom(data, level),
    }
}

/// Extract the parent of the "mpMRI" folder to load DICOMS according to the specified format.
///
/// Requires:
/// - path: the path to be analysed
///
/// Returns:
/// - the basepath before "mpMRI"
///
/// Example:
///
/// ```
/// # use slideslib::image_viewer::find_parent_of_mpmri;
/// # use std::path::PathBuf;
///
/// let par = PathBuf::from("/some/folder/mpMRI/subfolder/file.dcm");
/// let res = find_parent_of_mpmri(par.clone());
/// assert!(matches!(res, Some(_)));
/// assert_eq!(res.unwrap(), PathBuf::from("/some/folder/"));
/// ```
pub fn find_parent_of_mpmri(path: PathBuf) -> Option<PathBuf> {
    let mut components = path.components();

    let mut result = PathBuf::new();

    for comp in &mut components {
        let comp_str = comp.as_os_str();
        result.push(comp_str);

        if comp_str == "mpMRI" {
            // Remove the last component to get up to "MRI Test"
            result.pop(); // remove mpMRI
            return Some(result);
        }
    }
    None
}

/// GUI component for opening, viewing and processing openslide-compatible images. See [src/main.rs] for usage instructions for
/// usage instructions.
///
/// Requires the following fields:
///
///
/// - level: the current downscale factor
/// - max_level: the highest downscale factor
/// - dragging: if true, image is currently dragged
/// - drag_start: dragging start position
/// - offsetx: current x offset from center at full magnification
/// - offsety: current y offset from center at full magnification
/// - max_extents: image extents at full magnification
/// - image_path: a list of currently loaded images
/// - current_image: current image index
/// - script_path: currently selected script
/// - mouse_pos: current mouse position (point)
/// - cache_scale_factor: rate of cache size / viewport size
/// - theme: application theme
/// - mppx: x resolution in µm/px
/// - mppy: y resolution in µm/px
/// - info: list of current slide measurement infos
/// - plot_data: the plotting widget
/// - current_progress: current progress increased by, e.g., an executed script
/// - current_max_progress: the total maximum progress in steps
/// - current_info: index of info
/// - update_ready: thread safe indicator of finished loading
/// - loadtime_offsetx: x offset in the background cache (full magnification)
/// - loadtime_offsety: y offset in the background cache (full magnification)
/// - loadtime_cache: the threadsafe background cache
/// - levels: list of available downsample levels
/// - current_zoom: current relation of level / downsample (e.g., 15 / 16)
/// - current_extents: current image extents
/// - mask_active: if true, enable plotting of prediction created by script
/// - tracker: the wrapper for image position storing and calcuation
/// - current_border: currently selected border (Top, Left, ...)
/// - predictor: the wrapper to perform torchscript predictions
/// - show_pred: if true enable plotting of the prediction created by torchscript
/// - receiver: rx used for status communication in scripts
/// - sender: tx used for status communication in scripts
/// - draw: if true, render selection
/// - cur_sel: bounding points of the selection (roi)
/// - error: if available, current error status
/// - pred_thread_error: if available, current error status of torchlib predictor
/// - load_thread_error: if available, current error status of background loading
impl Application for ZoomableImageViewer {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        let cs: usize = CACHE_MAX as usize;

        // Initialise plot cache
        let cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();

        // Initialise prediction cache
        let mask_cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();

        // Initialise background cache
        let loadtime_cache_init = (Array::ones((cs, cs, 4)) * 255).into_owned().into_raw_vec();

        // Create references for the cache
        let cache = Rc::new(RefCell::new(cache_init));
        let mask_cache = Rc::new(RefCell::new(mask_cache_init));
        let theme = iced::Theme::custom(
            String::from("Dark"),
            theme::Palette {
                primary: Color::from([1., 1., 1.]),
                ..theme::Theme::Light.palette()
            },
        );

        let loadtime_cache = RefCell::new(loadtime_cache_init);

        let cs: u32 = cs as u32;
        let plot_data = SlideView::new(BaseViewArgs {
            cache_posx: 0.0,
            cache_posy: 0.0,
            cache_size: OpenslideSize { w: cs, h: cs },
            viewport_size: OpenslideSize {
                w: cs / 2,
                h: cs / 2,
            },
            viewport_default: OpenslideSize {
                w: cs / 2,
                h: cs / 2,
            },
            xoffset: Some(200),
            yoffset: Some(512),
            cache,
            mask_cache,
            mask_active: false,
            sel_start: None,
            sel_end: None,
            global_width: crate::WIDTH,
            global_height: crate::HEIGHT,
            cache_scale_factor_x: 2.,
            cache_scale_factor_y: 2.,
        });
        let (sender, receiver) = channel();
        let sf = 2.;

        let predictor_args = PredictorArgs {
            path: PathBuf::new(),
            width: cs / 2,
            height: cs / 2,
            depth: 0,
        };
        let defaults = (
            Self {
                level: 0,
                max_level: 0,
                dragging: false,
                drag_start: iced::Point { x: 0.0, y: 0.0 },
                plot_data,
                offsetx: 0.0,
                offsety: 0.0,
                max_extents: OpenslideSize { w: 512, h: 512 },
                image_path: Vec::from([PathBuf::new()]),
                current_image: 0,
                script_path: PathBuf::from("pyfunctions/count_objects.py"),
                mouse_pos: iced::Point { x: 0.0, y: 0.0 },
                cache_scale_factor_x: sf,
                cache_scale_factor_y: sf,
                theme,
                mppx: Vec::from([0.]),
                mppy: Vec::from([0.]),
                info: Vec::from([String::from(NOINFOTEXT)]),
                current_info: 0,
                current_progress: 0,
                current_max_progress: 1,
                update_ready: Arc::new(Mutex::new(false)),
                loadtime_offsetx: 0.,
                loadtime_offsety: 0.,
                loadtime_cache: Arc::new(Mutex::new(loadtime_cache)),
                levels: Vec::new(),
                current_zoom: 1.,
                current_extents: OpenslideSize { w: 512, h: 512 },
                mask_active: false,
                tracker: Tracker {
                    max_global_x: 1024.,
                    min_global_x: 0.,
                    max_global_y: 1024.,
                    min_global_y: 0.,
                    max_cache_x: 1024,
                    min_cache_x: 0,
                    max_cache_y: 1024,
                    min_cache_y: 0,
                    cache_size_x: 1024,
                    cache_size_y: 1024,
                    current_x: 512.,
                    current_y: 512.,
                    center_correction_x: 0.,
                    center_correction_y: 0.,
                    preload_possible: false,
                    cache_scale_factor_x: sf,
                    cache_scale_factor_y: sf,
                    cache_comp_x: 1.,
                    cache_comp_y: 1.,
                },
                current_border: Border {
                    cache: Borders::Center,
                    edge: Borders::Center,
                },
                predictor: SlidePredictor::new(predictor_args),
                show_pred: false,
                receiver: Arc::new(Mutex::new(receiver)),
                sender,
                draw: false,
                cur_sel: None,
                error: None,
                pred_thread_error: Arc::new(Mutex::new(None)),
                load_thread_error: Arc::new(Mutex::new(None)),
                on_border: false,
                imagetype: ImageType::WSI,
            },
            Command::none(),
        );
        prepare_freethreaded_python();
        defaults
    }
    fn theme(&self) -> Self::Theme {
        self.theme.clone()
    }

    fn title(&self) -> String {
        String::from("🍷OpenProsIT")
    }

    fn update(&mut self, _message: Message) -> Command<Message> {
        match _message {
            Message::ChooseFile(single) => {
                let default = PathBuf::from("./");
                let string_buf = self
                    .image_path
                    .get(self.current_image)
                    .unwrap_or(&default)
                    .parent()
                    .unwrap_or(Path::new("./"))
                    .as_os_str()
                    .to_str()
                    .unwrap_or("./");
                let start_path = &string_buf;
                let path = get_path("SVS Image", &["svs", "tiff", "dcm"], start_path, single);

                self.imagetype = ImageType::WSI;
                // Check if file is DICOM or WSI
                if path.to_str().unwrap_or("").contains(".dcm") {
                    self.imagetype = ImageType::DICOM;
                }
                // Check if folder contains DICOM
                let pattern = format!("{}/**/*.dcm", path.display());
                if glob(&pattern).map_or(false, |mut paths| paths.any(|entry| entry.is_ok())) {
                    self.imagetype = ImageType::DICOM;
                }
                self.info = Vec::new();
                self.image_path = Vec::new();
                self.image_path.push(path.clone());
                self.info.push(String::from(NOINFOTEXT));
                // Set path according to image type. List are not supported for DICOM.
                match self.imagetype {
                    ImageType::WSI => {
                        if path != PathBuf::from("") {
                            self.current_image = 0;
                            self.current_info = 0;
                            self.current_progress = 0;
                            if !single {
                                self.image_path.pop();
                                self.info.pop();

                                match get_file_list(path) {
                                    Ok(filelist) => {
                                        for subfile in filelist {
                                            match subfile {
                                                Ok(p) => {
                                                    self.image_path.push(p);
                                                    self.info.push(String::from(NOINFOTEXT));
                                                }
                                                _ => println!("Invalid path!"),
                                            }
                                        }
                                    }
                                    Err(err) => self.error = Some(err),
                                }
                            }
                        }
                        reset_offsets(self);
                    }
                    ImageType::DICOM => match find_parent_of_mpmri(path.clone()) {
                        Some(subpath) => {
                            self.image_path = Vec::new();
                            self.image_path.push(subpath);
                            self.current_image = 0;
                            self.current_info = 0;
                            self.current_progress = 0;
                        }
                        None => self.error = Some(ErrorKind::DicomImageLoadingError(path)),
                    },
                }
                if let Err(val) = load_data(self, None) {
                    self.error = Some(val);
                };
                Command::none()
            }
            Message::ChooseScript => {
                let string_buf = self
                    .script_path
                    .parent()
                    .unwrap_or(Path::new("./"))
                    .as_os_str()
                    .to_str()
                    .unwrap_or("./");
                let start_path = &string_buf;
                let path = get_path("Python File", &["py"], start_path, true);
                if path != PathBuf::from("") {
                    self.script_path = path.clone();
                    prepare_freethreaded_python();
                }
                Command::none()
            }
            Message::RunScript => {
                self.current_max_progress = self.image_path.len();
                let script_name = self
                    .script_path
                    .file_name()
                    .unwrap_or(OsStr::new("increase"))
                    .to_str()
                    .unwrap_or("increase");

                {
                    let file_name = String::from(&script_name[..script_name.len() - 3]);
                    if let Err(val) = load_data(self, Some(self.level)) {
                        self.error = Some(val);
                        return Command::none();
                    };
                    let bounds = get_viewport_bounds(&self.plot_data.view);
                    let mut width = self.plot_data.view.cache_size.w as usize;
                    let mut height = self.plot_data.view.cache_size.h as usize;
                    let cache = self.plot_data.view.cache.borrow();
                    let array;
                    match ArrayView::from_shape((height, width, 4), &cache) {
                        Ok(val) => array = val,
                        Err(err) => {
                            self.error = Some(ErrorKind::ArrayError(file_name, err.to_string()));
                            return Command::none();
                        }
                    }
                    width = bounds.width as usize;
                    height = bounds.height as usize;
                    let flat_vec = array
                        .slice(s!(
                            bounds.y as usize..bounds.y as usize + height,
                            bounds.x as usize..bounds.x as usize + width,
                            0..4
                        ))
                        .into_owned()
                        .into_raw_vec();
                    let script_path = self
                        .script_path
                        .parent()
                        .unwrap_or(Path::new("./"))
                        .to_str()
                        .unwrap_or("./");
                    let info;
                    match execute_script_for_file(
                        self,
                        &flat_vec,
                        width,
                        height,
                        file_name.clone(),
                        String::from(script_path.to_string()),
                        String::from(self.image_path[self.current_image].to_str().unwrap_or("")),
                    ) {
                        Ok((info_, _)) => info = info_,
                        Err(err) => {
                            self.error = Some(err);
                            return Command::none();
                        }
                    };
                    self.info[self.current_info] = info;
                    self.current_image += 1;
                    self.current_info += 1;
                }
                if self.current_progress == 0 {
                    self.current_image = 0;
                    self.current_info = 0;
                };
                if self.current_progress > self.image_path.len() {
                    self.current_progress = 1
                };
                if self.current_image >= self.image_path.len() {
                    self.current_image -= 1;
                    self.current_info -= 1;
                };
                self.current_progress += 1;

                match self.current_progress < self.image_path.len() {
                    true => Command::perform(async {}, |()| Message::RunScript),
                    _ => {
                        self.current_progress = 0;
                        if let Err(val) = load_data(self, Some(self.level)) {
                            self.error = Some(val);
                        };
                        Command::none()
                    }
                }
            }
            Message::KeyPressed(key_code) => {
                // Logic for DICOM
                if matches!(self.imagetype, ImageType::DICOM) {
                    match key_code {
                        Key::Named(Named::ArrowUp) => {
                            if self.level < self.max_level {
                                self.level += 1
                            }
                            true
                        }
                        Key::Named(Named::ArrowDown) => {
                            if self.level >= 1 {
                                self.level -= 1
                            }
                            true
                        }
                        _ => false,
                    };
                } else {
                    let old_level = self.level;
                    let is_arrow = match key_code {
                        Key::Named(Named::ArrowDown) => {
                            if self.level < self.max_level {
                                if self.level < 4 {
                                    self.level += 1
                                } else {
                                    self.level += STEP;
                                }
                            }
                            true
                        }
                        Key::Named(Named::ArrowUp) => {
                            if self.level > 4 {
                                self.level -= STEP;
                            } else {
                                self.level -= 1
                            }
                            if self.level < 1 {
                                self.level = 1
                            }
                            true
                        }
                        _ => false,
                    };
                    if is_arrow {
                        if self.level == self.max_level {
                            reset_offsets(self);
                            self.error = update_zoom_props(self);
                            self.error = update_cache_data(self, false, self.imagetype);
                        } else {
                            let (_, level) =
                                find_next_greater_value(self.levels.clone(), self.level)
                                    .unwrap_or((0, self.max_level));
                            self.error = update_zoom_props(self);
                            update_offsets(self, old_level);
                            let (_, old_level) =
                                find_next_greater_value(self.levels.clone(), old_level)
                                    .unwrap_or((0, self.max_level));
                            if level != self.max_level || old_level != self.max_level {
                                self.error = update_cache_data(self, false, self.imagetype);
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::DragStart => {
                self.dragging = true;
                Command::none()
            }
            Message::DragEnd => {
                self.dragging = false;
                Command::none()
            }
            Message::MouseMove(pos) => {
                if matches!(self.imagetype, ImageType::DICOM) {
                    return Command::none(); //DICOM has no interactions
                }
                if self.dragging & self.draw & !self.plot_data.view.sel_start.is_some() {
                    self.plot_data.view.sel_start = Some(pos);
                    return Command::none();
                }
                if self.draw {
                    self.plot_data.view.sel_end = Some(pos);
                    if !self.dragging
                        & self.plot_data.view.sel_start.is_some()
                        & self.plot_data.view.sel_end.is_some()
                    {
                        self.draw = false;
                        self.cur_sel = self.plot_data.view.get_selection_bounds();
                        return Command::none();
                    }
                }
                if self.dragging & (self.level < self.max_level) {
                    let new_mouse_pos = pos;
                    if new_mouse_pos.distance(self.mouse_pos) > 1.0 {
                        self.mouse_pos = new_mouse_pos;
                        let delta = pos - self.drag_start;
                        self.drag_start = pos;

                        let (_, level) = find_next_greater_value(self.levels.clone(), self.level)
                            .unwrap_or((0, self.level));
                        let limits: Limits = self.tracker.update_coords(
                            self.level as u32,
                            level as u32,
                            self.offsetx.borrow_mut(),
                            self.offsety.borrow_mut(),
                            self.plot_data.view.cache_posx.borrow_mut(),
                            self.plot_data.view.cache_posy.borrow_mut(),
                            delta.x,
                            delta.y,
                        );
                        let border = self.tracker.get_current_border(&limits);
                        let mut is_edge = match border {
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
                        if (border != Borders::Center)
                            & (self.current_border.cache != border)
                            & self.tracker.preload_possible
                            & !is_edge
                        {
                            self.current_border.cache = border.clone();
                            if !is_edge {
                                self.current_border.edge = border.clone();
                            }
                            self.error = update_zoom_props(self);
                            self.error = update_cache_data(self, true, self.imagetype);
                        }
                        is_edge = match self.current_border.cache {
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

                        if limits.border_reached & !is_edge & self.tracker.preload_possible {
                            self.current_border.edge = border.clone();
                            self.current_border.cache = Borders::Center;
                            change_cache(self, true);
                        }
                    }
                } else {
                    self.drag_start = pos;
                }
                Command::none()
            }
            Message::OnVerResize(position) => {
                self.plot_data.view.yoffset = Some(position);
                Command::none()
            }
            Message::OnHorResize(position) => {
                self.plot_data.view.xoffset = Some(position);
                Command::none()
            }
            Message::ChangeFile(idx) => {
                if let Err(err) = change_file(self, idx) {
                    self.error = Some(err)
                }
                Command::none()
            }
            Message::RunPrediction(dims) => {
                self.current_progress = 0;
                let path = String::from(self.image_path[self.current_image].to_str().unwrap_or(""));

                let args = PredictorArgs {
                    path: PathBuf::from(path.as_str()),
                    width: self.plot_data.view.viewport_size.w,
                    height: self.plot_data.view.viewport_size.h,
                    depth: self.max_level,
                };
                let mut predictor: Box<dyn Predictor>;
                match self.imagetype {
                    ImageType::WSI => {
                        predictor = Box::new(match SlidePredictor::new(args.clone()) {
                            Ok(val) => val,
                            Err(err) => {
                                self.error = Some(err);
                                return Command::none();
                            }
                        });
                    }
                    ImageType::DICOM => {
                        predictor = Box::new(match DicomPredictor::new(args.clone()) {
                            Ok(val) => val,
                            Err(err) => {
                                self.error = Some(err);
                                return Command::none();
                            }
                        });
                    }
                }
                //let mut predictor = Arc::new(predictor);
                if let Err(err) = predictor.preprocess() {
                    self.error = Some(err);
                    return Command::none();
                };
                self.current_max_progress = predictor.max_progress();
                let tx = self.sender.clone();
                let imagetype = self.imagetype;
                let thread_error_arc = Arc::clone(&self.pred_thread_error);
                std::thread::spawn(move || match imagetype {
                    ImageType::WSI => match SlidePredictor::new(args.clone()) {
                        Ok(mut predictor_) => {
                            if let Err(err) = predictor_.run(None, dims, tx) {
                                log_or_load_thread_err(thread_error_arc, Some(err));
                            };
                        }
                        Err(err) => {
                            log_or_load_thread_err(thread_error_arc, Some(err));
                        }
                    },
                    ImageType::DICOM => match DicomPredictor::new(args.clone()) {
                        Ok(mut predictor_) => {
                            if let Err(err) = predictor_.run(None, dims, tx) {
                                log_or_load_thread_err(thread_error_arc, Some(err));
                            };
                        }
                        Err(err) => {
                            log_or_load_thread_err(thread_error_arc, Some(err));
                        }
                    },
                });

                Command::none()
            }
            Message::TogglePred => {
                let path = String::from(self.image_path[self.current_image].to_str().unwrap_or(""));

                match self.imagetype {
                    ImageType::WSI => {
                        let out_path = replace_suffix_with_pred(
                            path.as_str(),
                         );
                        self.error = match wait_until_file_ready(out_path.as_str(), 10) {
                            Err(err) => Some(ErrorKind::VipsOpError(String::from("Writing Error"), err.to_string()).into()),
                            _ => {
                                self.show_pred = !self.show_pred;
                                None
                            }
                        };
                    },
                    ImageType::DICOM => {
                        let pred = PathBuf::from(path.clone()).join("pred.npy");
                        let out_path = pred.as_os_str().to_str().unwrap_or(path.as_str());
                        self.error = match wait_until_file_ready(out_path, 10) {
                            Err(err) => Some(ErrorKind::VipsOpError(String::from("Writing Error"), err.to_string()).into()),
                            _ => {
                                self.mask_active = !self.mask_active;
                                self.show_pred = !self.show_pred;       
                                None                     
                            }
                        };
                    }
                }

                self.error = update_cache_data(self, false, self.imagetype);
                Command::none()
            }
            Message::UpdateCounter => {
                self.current_progress += 1;
                if self.current_progress == self.current_max_progress {
                    return Command::perform(async {}, |_| Message::TogglePred);
                }
                Command::none()
            }
            Message::Crop => {
                self.plot_data.view.sel_start = None;
                self.plot_data.view.sel_end = None;
                self.cur_sel = None;
                self.draw = true;
                Command::none()
            }
            Message::WindowResized((w, h)) => {
                self.plot_data.view.global_width = w;
                self.plot_data.view.global_height = h;
                Command::none()
            }
            Message::HideModal => match self.error {
                Some(ErrorKind::NoFileError()) => {
                    Command::perform(async {}, |_| Message::ChooseFile(true))
                }
                _ => {
                    self.error = None;
                    reset_thread_err(&self.pred_thread_error);
                    reset_thread_err(&self.load_thread_error);
                    Command::none()
                }
            },

            _ => Command::none(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let ui_subscriptions = event::listen_with(|event, _| match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Message::MouseMove(position))
            }
            Event::Mouse(mouse::Event::ButtonPressed(_)) => Some(Message::DragStart),
            Event::Mouse(mouse::Event::ButtonReleased(_)) => Some(Message::DragEnd),
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: key_code,
                modifiers: _,
                location: Location::Standard,
                text: None,
            }) => Some(Message::KeyPressed(key_code)),
            Event::Window(_, window::Event::Resized { width, height }) => {
                Some(Message::WindowResized((width, height)))
            }
            _ => None,
        });

        let mut subscriptions = Vec::new();

        let receiver = Arc::clone(&self.receiver);
        let clf_subscription =
            iced::Subscription::from_recipe(CounterUpdateSubscription { receiver });
        subscriptions.push(ui_subscriptions);
        subscriptions.push(clf_subscription);
        // Start subscription to receive messages from background thread
        Subscription::batch(subscriptions)
    }

    fn view(&self) -> Element<Message> {
        // the slide select section
        let mut slide_row: Vec<Button<_>> = Vec::new();
        let mut no_file: bool = true;
        for i in 0..self.image_path.len() {
            let alternate_name = format!("No file available!");
            let mut filename = self.image_path[i]
                .file_name()
                .unwrap_or(OsStr::new(alternate_name.as_str()))
                .to_str()
                .unwrap_or(alternate_name.as_str());
            no_file = filename == alternate_name;
            let short_filename;
            if filename.len() > 20 {
                short_filename = format!("{}..{}", &filename[..6], &filename[filename.len() - 6..]);
                filename = short_filename.as_str();
            };
            slide_row.push(labeled_list_button(filename, Some(Message::ChangeFile(i))).height(20))
        }
        let slides_: Vec<Element<_>> = slide_row.into_iter().map(Element::from).collect();
        let slides = scrollable(Column::with_children(slides_));
        let mut pred_available = false;
        let mut error = self.error.clone();
        match self.image_path.get(self.current_image) {
            Some(p) => {
                pred_available =
                    PathBuf::from(replace_suffix_with_pred(p.to_str().unwrap_or(""))).exists();
            }
            None => {
                error = Some(ErrorKind::NoFileError());
            }
        }

        // the topbar buttons
        let is_mri = matches!(self.imagetype, ImageType::DICOM);
        let buttons = row!(
            labeled_button(
                "Crop",
                if no_file || is_mri {
                    None
                } else {
                    Some(Message::Crop)
                }
            )
            .width(75),
            labeled_button(
                "Analyse",
                if no_file {
                    None
                } else {
                    Some(Message::RunScript)
                },
            )
            .width(75),
            labeled_button(
                "Classify Image",
                if no_file {
                    None
                } else {
                    Some(Message::RunPrediction(None))
                },
            )
            .width(100),
            labeled_button(
                if self.show_pred {
                    "AI Map Off"
                } else {
                    "AI Map On "
                },
                if no_file | !pred_available {
                    None
                } else {
                    Some(Message::TogglePred)
                },
            )
            .width(75),
        );
        let menu_bar = default_menu();
        let topbar_style: fn(&iced::Theme) -> iced::widget::container::Appearance =
            |theme| TopbarStyle.appearance(&theme);
        let topbar = Container::new(
            row!(menu_bar, buttons)
                .align_items(iced::Alignment::Center)
                .spacing(4.),
        )
        .style(topbar_style)
        .width(Length::Fill)
        .height(40);
        let image_widget: Element<Message, Theme, iced::Renderer>;

        // image viewer
        match self.imagetype {
            ImageType::WSI => {
                image_widget = SlideView::new(BaseViewArgs::new(
                    self.plot_data.view.cache.clone(),
                    self.plot_data.view.mask_cache.clone(),
                    self.plot_data.view.viewport_size,
                    self.plot_data.view.viewport_default,
                    self.plot_data.view.cache_size,
                    self.plot_data.view.cache_posx,
                    self.plot_data.view.cache_posy,
                    self.plot_data.view.xoffset,
                    self.plot_data.view.yoffset,
                    self.mask_active,
                    self.plot_data.view.sel_start,
                    self.plot_data.view.sel_end,
                    self.plot_data.view.global_width,
                    self.plot_data.view.global_height,
                    self.plot_data.view.cache_scale_factor_x,
                    self.plot_data.view.cache_scale_factor_y,
                ))
                .into();
            }
            ImageType::DICOM => {
                image_widget = DicomView::new(
                    BaseViewArgs::new(
                        self.plot_data.view.cache.clone(),
                        self.plot_data.view.mask_cache.clone(),
                        self.plot_data.view.viewport_size,
                        self.plot_data.view.viewport_default,
                        self.plot_data.view.cache_size,
                        self.plot_data.view.cache_posx,
                        self.plot_data.view.cache_posy,
                        self.plot_data.view.xoffset,
                        self.plot_data.view.yoffset,
                        self.mask_active,
                        self.plot_data.view.sel_start,
                        self.plot_data.view.sel_end,
                        self.plot_data.view.global_width,
                        self.plot_data.view.global_height,
                        self.plot_data.view.cache_scale_factor_x,
                        self.plot_data.view.cache_scale_factor_y,
                    ),
                    self.level as usize,
                )
                .into();
            }
        }

        // measurement info and layout divider
        let info = Container::new(
            Text::new(
                self.info
                    .get(self.current_info)
                    .cloned()
                    .unwrap_or(String::from(NOINFOTEXT).clone()),
            )
            .size(12),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y();
        let divider = Split::new(
            Split::new(
                slides,
                info,
                self.plot_data.view.yoffset,
                split::Axis::Horizontal,
                Message::OnVerResize,
            ),
            image_widget,
            self.plot_data.view.xoffset,
            split::Axis::Vertical,
            Message::OnHorResize,
        );

        let mut main_layout = Column::new();

        main_layout = main_layout.push(topbar).push(divider);

        main_layout = main_layout.push(
            progress_bar(
                0.0..=100.0,
                (self.current_progress as f32 / self.current_max_progress as f32) * 100.,
            )
            .height(5)
            .style(iced::theme::ProgressBar::Custom(Box::new(ProgressStyle {
                0: iced_aw::style::colors::PRIMARY,
            }))),
        );
        let mut content = Container::new(main_layout)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into();

        let thread_error_arc = Arc::clone(&self.pred_thread_error);
        let pred_error = log_or_load_thread_err(thread_error_arc, None);
        let thread_error_arc = Arc::clone(&self.load_thread_error);
        let load_error = log_or_load_thread_err(thread_error_arc, None);
        content = match (load_error, pred_error, error) {
            (Some(val), _, _) | (_, Some(val), _) | (_, _, Some(val)) => match val {
                ErrorKind::OpenSlideImageLoadingError(ref path)
                | ErrorKind::DicomImageLoadingError(ref path) => {
                    let mut content_ = content;
                    if path != &PathBuf::from("") {
                        content_ = Modal::new(content_, modal(val.to_string()))
                            .on_blur(Message::HideModal)
                            .into()
                    }
                    content_
                }
                _ => Modal::new(content, modal(val.to_string()))
                    .on_blur(Message::HideModal)
                    .into(),
            },
            (None, None, None) => content,
        };
        content
    }
}
