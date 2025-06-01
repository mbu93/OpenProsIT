#[cfg(test)]
mod integration_tests {

    const IMAGE_PATCHES: u8 = 1;
    use glob::glob;
    // iced
    use iced::{
        keyboard::{key::Named, Key},
        Application, Command, Point,
    };

    // Libvips
    use libvips::{VipsApp, VipsImage};
    use openslide_rs::Size;

    // STD lib
    use std::{
        path::PathBuf,
        sync::Arc,
        vec::Vec,
    };

    // Python bindings
    use pyo3::prepare_freethreaded_python;

    // Lazy static
    use lazy_static::lazy_static;

    // Internal modules
    use slideslib::{
        cache::{reset_offsets, update_cache_data, update_zoom_props},
        error::ErrorKind,
        gui_components::Message,
        image_viewer::{load_data, NOINFOTEXT},
        predictor::PreprocessingDims,
        slide_predictor::replace_suffix_with_pred,
        util::{get_file_list, log_or_load_thread_err},
        ImageType, ZoomableImageViewer,
    };

    lazy_static! {
        static ref VIPS_APP: VipsApp = {
            let app = VipsApp::new("RustyVips", false).expect("Cannot init VIPS");
            app.concurrency_set(1);
            app
        };
    }
    struct MockViewer {
        viewer: ZoomableImageViewer,
    }
    impl MockViewer {
        fn new() -> Self {
            return Self {
                viewer: ZoomableImageViewer::new(()).0,
            };
        }
        fn update(&mut self, _message: Message) -> Command<Message> {
            match _message {
                Message::ChooseFile(single) => {
                    let path = match self.viewer.imagetype {
                        ImageType::WSI => {
                            PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.tiff")
                        }
                        ImageType::DICOM => PathBuf::from("tests").join("MRI Test"),
                    };

                    self.viewer.imagetype = ImageType::WSI;
                    // Check if file is DICOM or WSI
                    if path.to_str().unwrap_or("").contains(".dcm") {
                        self.viewer.imagetype = ImageType::DICOM;
                    }
                    // Check if folder contains DICOM
                    let pattern_ = path.join("**").join("*.dcm");
                    let pattern = pattern_.as_os_str().to_str().unwrap_or("");
                    if glob(&pattern).map_or(false, |mut paths| paths.any(|entry| entry.is_ok())) {
                        self.viewer.imagetype = ImageType::DICOM;
                    }
                    self.viewer.info = Vec::new();
                    self.viewer.image_path = Vec::new();
                    self.viewer.image_path.push(path.clone());
                    self.viewer.info.push(String::from(NOINFOTEXT));
                    // Set path according to image type. List are not supported for DICOM.
                    match self.viewer.imagetype {
                        ImageType::WSI => {
                            if path != PathBuf::from("") {
                                self.viewer.current_image = 0;
                                self.viewer.current_info = 0;
                                self.viewer.current_progress = 0;
                                if !single {
                                    self.viewer.image_path.pop();
                                    self.viewer.info.pop();

                                    match get_file_list(path) {
                                        Ok(filelist) => {
                                            for subfile in filelist {
                                                match subfile {
                                                    Ok(p) => {
                                                        self.viewer.image_path.push(p);
                                                        self.viewer
                                                            .info
                                                            .push(String::from(NOINFOTEXT));
                                                    }
                                                    _ => println!("Invalid path!"),
                                                }
                                            }
                                        }
                                        Err(err) => self.viewer.error = Some(err),
                                    }
                                }
                            }
                            reset_offsets(&mut self.viewer);
                        }
                        _ => {
                            self.viewer.image_path = Vec::new();
                            self.viewer.image_path.push(PathBuf::from("tests").join("MRI Test"));
                            self.viewer.current_image = 0;
                            self.viewer.current_info = 0;
                            self.viewer.current_progress = 0;
                        }
                    }

                    if let Err(val) = load_data(&mut self.viewer, None) {
                        self.viewer.error = Some(val);
                    };
                    Command::none()
                }
                Message::ChooseScript => {
                    let path = PathBuf::from("pyfunctions").join("measurement_mock.py");
                    if path != PathBuf::from("") {
                        self.viewer.script_path = path.clone();
                        prepare_freethreaded_python();
                    }
                    Command::none()
                }
                _ => self.viewer.update(_message),
            }
        }
    }
    #[test]
    fn open_slide() {
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::WSI;
        let cache_arc = viewer.viewer.plot_data.view.cache.clone();
        let cache_data = cache_arc.clone();
        let sum: u32 = cache_data.borrow().iter().map(|x| *x as u32).sum();
        assert_eq!(sum, 267386880);
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::WSI;
        let _ = viewer.update(Message::ChooseFile(true));
        assert_eq!(
            viewer.viewer.image_path[viewer.viewer.current_image],
            PathBuf::from("tests").join("data").join("02a7b258e875cf073e2421d67ff824cd.tiff")
        );
        let cache_arc = viewer.viewer.plot_data.view.cache.clone();
        let cache_data = cache_arc.clone();
        let sum: u32 = cache_data.borrow().iter().map(|x| *x as u32).sum();
        assert_ne!(sum, 267386880);
    }
    #[test]
    fn open_dicom() {
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::DICOM;
        let cache_arc = viewer.viewer.plot_data.view.cache.clone();
        let cache_data = cache_arc.clone();
        let sum: u32 = cache_data.borrow().iter().map(|x| *x as u32).sum();
        assert_eq!(sum, 267386880);
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::DICOM;
        let _ = viewer.update(Message::ChooseFile(true));
        assert_eq!(
            viewer.viewer.image_path[viewer.viewer.current_image],
            PathBuf::from("data").join("preprocessed").join("MRI Test")
        );
        let cache_arc = viewer.viewer.plot_data.view.cache.clone();
        let cache_data = cache_arc.clone();
        let sum: u32 = cache_data.borrow().iter().map(|x| *x as u32).sum();
        assert_ne!(sum, 267386880);
    }
    #[test]
    fn run_script() -> Result<(), ErrorKind> {
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::WSI;
        let _ = viewer.update(Message::ChooseFile(true));
        let _ = viewer.update(Message::ChooseScript);
        let _ = viewer.update(Message::RunScript);
        assert_ne!(viewer.viewer.current_max_progress, 0);
        match viewer.viewer.error {
            Some(err) => return Err(err),
            None => assert_eq!(
                viewer.viewer.info.get(0).unwrap(),
                &String::from("testfield: 240.97769\n")
            ),
        };
        Ok::<(), ErrorKind>(())
    }

    #[test]
    fn run_wsi_pred() -> Result<(), ErrorKind> {
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::WSI;
        let _ = viewer.update(Message::ChooseFile(true));

        let thread_error_arc = Arc::clone(&viewer.viewer.pred_thread_error);
        let pred_error = log_or_load_thread_err(thread_error_arc, None);
        if let Some(err) = pred_error {
            println!("{:?}", err);
        }
        let preprocessing_data = PreprocessingDims {
            owidth: 1120,
            oheight: 1120,
            nwidth: 1120,
            nheight: 1120,
            outdims: Size { w: 70, h: 70 },
        };
        let _ = viewer.update(Message::RunPrediction(Some(preprocessing_data)));

        if let Some(err) = viewer.viewer.error {
            println!("{:?}", err);
        }
        viewer.viewer.show_pred = false;

        let rx = viewer.viewer.receiver.lock().unwrap();
        let mut counter = 0;
        while counter < IMAGE_PATCHES {
            match rx.recv() {
                Ok(Message::UpdateCounter) => {
                    println!("Patch processed!");
                    counter += 1;
                }
                _ => {}
            }
        }
        let thread_error_arc = Arc::clone(&viewer.viewer.pred_thread_error);
        let pred_error = log_or_load_thread_err(thread_error_arc, None);
        assert!(
            matches!(pred_error, None),
            "Error encountered in prediction!"
        );
        let p = viewer.viewer.image_path[viewer.viewer.current_image].to_str();
        let pred_path = replace_suffix_with_pred(p.unwrap());
        let img = VipsImage::new_from_file(pred_path.as_str());
        let data = img
            .map_err(|err| ErrorKind::VipsOpError(pred_path, err.to_string()))?
            .image_write_to_memory();
        let sum: u32 = data.iter().map(|x| *x as u32).sum();
        assert_eq!(sum, 3746013);

        Ok::<(), ErrorKind>(())
    }

    #[test]
    fn zoom() -> Result<(), ErrorKind> {
        // Test zoom for slide. Note that DICOMs are loaded in 3D, so testing the zoom is of low
        // value and thus was omitted as the test case would be immense.
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::WSI;
        let _ = viewer.update(Message::ChooseFile(true));
        let data = viewer.viewer.plot_data.view.cache.borrow().clone();
        let sum_pre: u32 = data.iter().map(|x| *x as u32).sum();

        // Zoom in to magnification 4
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));

        let data = viewer.viewer.plot_data.view.cache.borrow();
        let sum_post: u32 = data.iter().map(|x| *x as u32).sum();
        assert_ne!(sum_pre, sum_post);

        Ok::<(), ErrorKind>(())
    }

    #[test]
    fn drag_and_update() -> Result<(), ErrorKind> {
        let mut viewer = MockViewer::new();
        viewer.viewer.imagetype = ImageType::WSI;
        let _ = viewer.update(Message::ChooseFile(true));

        // Zoom in to magnification 4
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));
        let _ = viewer.update(Message::KeyPressed(Key::Named(Named::ArrowUp)));
        let data = viewer.viewer.plot_data.view.cache.borrow().clone();
        let sum_pre: u32 = data.iter().map(|x| *x as u32).sum();
        let _ = viewer.update(Message::DragStart);
        let _ = viewer.update(Message::MouseMove(Point::new(500., 500.)));
        let _ = viewer.update(Message::DragEnd);
        update_zoom_props(&mut viewer.viewer);
        update_cache_data(&mut viewer.viewer, false, ImageType::WSI);
        let data = viewer.viewer.plot_data.view.cache.borrow();
        let sum_post: u32 = data.iter().map(|x| *x as u32).sum();
        assert_ne!(sum_pre, sum_post);
        Ok::<(), ErrorKind>(())
    }
}
