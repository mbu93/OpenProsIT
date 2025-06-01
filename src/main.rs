//! The executable wiki for wikilib
extern crate glob;
extern crate iced_aw;
extern crate iced_style;
extern crate libvips;
extern crate native_dialog;
extern crate ndarray;
extern crate ndarray_npy;
extern crate openslide_rs;
extern crate rand;
extern crate rfd;
extern crate slideslib;
extern crate lazy_static;
extern crate npyz;
use iced::{Application, Settings, Size};
use libvips::VipsApp;

use slideslib::{ZoomableImageViewer, HEIGHT, WIDTH};
/// The runtime. Invokes an instance of VIPS that handles image processing and
/// invokes an instance if the ZoomableImageViewer that can be run as follows:
/// ```
/// extern crate glob;
/// extern crate openslide_rs;
/// extern crate slideslib;
/// extern crate error_chain;
/// extern crate iced_aw;
/// extern crate iced_style;
/// extern crate ndarray;
/// extern crate libvips;
/// extern crate native_dialog;
/// extern crate rfd;
/// extern crate ndarray_npy;
/// extern crate rand;
/// use iced::{Application, Settings, Size};
/// use libvips::VipsApp;
///
/// use slideslib::{ZoomableImageViewer, HEIGHT, WIDTH};
/// use iced::{Settings};
///
/// let mut settings = Settings::default();
/// settings.window.size = Size {
///     width: 800.,
///     height: 600.,
/// };
/// let _ = ZoomableImageViewer::run(settings);
/// ```
/// whereas the settings specify the width and height of the UI (default: 800, 600)
fn main() {
    let app = VipsApp::new("RustyVips", false).expect("Cannot init VIPS");
    app.concurrency_set(16);

    let mut settings = Settings::default();
    settings.window.size = Size {
        width: WIDTH as f32,
        height: HEIGHT as f32,
    };
    let _ = ZoomableImageViewer::run(settings);
}
