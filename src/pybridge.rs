use crate::error::ErrorKind;
use crate::ZoomableImageViewer;
use libvips::{ops, VipsImage};
use pyo3::prelude::{PyModule, PyResult, Python};
use pyo3::types::PyAnyMethods;
/// Execute a script for the currently selected slide and according to the script selection
/// (program default: count_objects.py). Will retrieve an error if the script crashes or can't be
/// executed. Will return an information String for both Overlay and Measurement script types,
/// whereas Overlay will cause an empty info String and the mask_cache of the slideviewer to be
/// updated with the returned overlay's values.
///
/// Example
///
/// ```
/// # use ndarray::{Array, s, Ix3};
/// # use iced::application::Application;
/// # use slideslib::{pybridge::execute_script_for_file, ZoomableImageViewer, error::ErrorKind};
/// # use ndarray::ShapeBuilder;
/// # use std::path::PathBuf;
/// # use pyo3::{PyErr, prepare_freethreaded_python};
/// # prepare_freethreaded_python();
/// let viewer = ZoomableImageViewer::new(()).0;
///
/// let mut image_data = Array::<u8, Ix3>::ones((50, 50, 4))*255;
/// let mut slice = image_data.slice_mut(s![25.., 25.., 1..2]);
/// slice.fill(0);
/// let vec: Vec<u8> = image_data.into_raw_vec();
///
/// // Test if measurment script can be executed.
/// let (info, plot) = execute_script_for_file(&viewer, &vec, 50, 50, "measurement_mock".into(), "pyfunctions".into(), 
/// PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_str().unwrap_or("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_string())?;
/// assert_eq!(info, "testfield: 239.0625\n");
/// assert_eq!(plot, false);
///
/// // Test if overlay script can be executed.
/// let (info, plot) = execute_script_for_file(&viewer, &vec, 50, 50, "overlay_mock".into(), "pyfunctions".into(), 
/// PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_str().unwrap_or("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_string())?;
/// let arr_mean: u32 = vec.iter().map(|x| *x as u32).sum();
/// let viewer_cache_mean: u32 = (*viewer.plot_data.view.mask_cache.borrow()).iter().map(|x| *x as u32).sum();
/// assert_eq!(plot, true);
/// assert_eq!(info, "");
///
/// // Test if non-existent script causes catched error.
/// assert!(matches!(execute_script_for_file(&viewer, &vec, 50, 50, "not_existent".into(), "pyfunctions".into(), 
/// PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_str().unwrap_or("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_string()).unwrap_err(), ErrorKind), "Error was not captured!");
///
/// // Test if script error (wrong size provided) causes catched error.
/// assert!(matches!(execute_script_for_file(&viewer, &vec, 35, 50, "measurement_mock".into(), "pyfunctions".into(), 
/// PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_str().unwrap_or("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_string()).unwrap_err(), ErrorKind), "Error was not captured!");
/// Ok::<(), ErrorKind>(())
/// ```
pub fn execute_script_for_file(
    data: &ZoomableImageViewer,
    flat_vec: &Vec<u8>,
    width: usize,
    height: usize,
    file_name: String,
    script_path: String,
    in_path: String,
) -> Result<(String, bool), ErrorKind> {
    let mut info = format!("");
    let mut plot: bool = false;
    let roi = match data.cur_sel {
        Some(sel) => [
            sel.x as i64,
            (sel.x + sel.width) as i64,
            sel.y as i64,
            (sel.y + sel.height) as i64,
        ],
        _ => [0, -1, 0, -1],
    };
    match run_script(
        &flat_vec,
        width as u32,
        height as u32,
        4,
        file_name.clone(),
        script_path.clone(),
        data.mppx.last().unwrap_or(&0.) / data.level as f32,
        data.mppy.last().unwrap_or(&0.) / data.level as f32,
        roi,
        String::from(
            data.image_path[data.current_image]
                .as_os_str()
                .to_str()
                .unwrap_or("./"),
        ),
        in_path,
    ) {
        Ok(value) => {
            if value.script_type.to_lowercase() == "measurement" {
                let measuremnts = value
                    .output
                    .iter()
                    .map(|&f| f.to_string())
                    .collect::<Vec<String>>();
                let names = value.field_names;
                for (meas, name) in measuremnts.iter().zip(names) {
                    info.push_str(format!("{}: {}\n", name, meas).as_str());
                }
            }
            if value.script_type.to_lowercase() == "overlay" {
                plot = true;
                let overlay: Vec<u8> = value.output.iter().map(|&x| (x * 255.) as u8).collect();
                let mut overlay_img = VipsImage::new_from_memory(
                    overlay.as_slice(),
                    width as i32,
                    height as i32,
                    4,
                    ops::BandFormat::Uchar,
                )
                .map_err(|err| ErrorKind::VipsOpError(file_name.clone(), err.to_string()).into())?;
                overlay_img = ops::gravity(
                    &overlay_img,
                    ops::CompassDirection::Centre,
                    data.plot_data.view.cache_size.w as i32,
                    data.plot_data.view.cache_size.h as i32,
                )
                .map_err(|err| ErrorKind::VipsOpError(file_name.clone(), err.to_string()).into())?;
                let cache = data.plot_data.view.cache.borrow();
                let base_img = VipsImage::new_from_memory(
                    &cache,
                    data.plot_data.view.cache_size.w as i32,
                    data.plot_data.view.cache_size.h as i32,
                    4,
                    ops::BandFormat::Uchar,
                )
                .map_err(|err| ErrorKind::VipsOpError(file_name.clone(), err.to_string()).into())?;
                let _composite = ops::composite_2(&overlay_img, &base_img, ops::BlendMode::Overlay)
                    .unwrap_or(base_img);
                data.plot_data
                    .view
                    .mask_cache
                    .replace(overlay_img.image_write_to_memory());
            }
        }
        Err(err) => {
            return Err(ErrorKind::ScriptError(
                file_name,
                String::from(script_path),
                err.to_string(),
            )
            .into());
        }
    };
    Ok((info, plot))
}
pub struct PythonResponse {
    pub output: Vec<f32>,
    pub field_names: Vec<String>,
    pub script_type: String,
}
/// Execute a script located in "pyfunctions" or any folder you specify by selecting a script in
/// the GUI. For successful execution, the script requires a function call of the following
/// signature:
/// ```ignore
/// def call(
///     obj: List[np.uint8],
///     width: np.uint32,
///     height: np.uint32,
///     channels: np.uint8,
///     mppx: float,
///     mppy: float,
///     roi: List[np.int64],
///     outpath: str,
/// ) -> Tuple[List[float], List[str]]:
///     output = somefunction(obj, width, height)
///     return (
///         [area * 1e-6, count, (area / total_area) * 100],
///         ["Tissue (mm)²", "Nr. Objects", "Tissue/Total (%)"],
///     )
/// ```
/// whereas obj is the bytevec of the currently selected roi. The function returns two lists of values
/// and keys, e.g., to be rendered in the info field of the application. To call the run_script
/// function, pyo3 needs to be readily initialised. The script may also have a global attribute
/// TYPE that specifies whether a "Measurement" (default) or an "Overlay" is returned. In the
/// ZoomableImageViewer this will be used to either pipe the output either to the info box or to trigger
/// the rendering of the result.
/// The following arguments are required:
/// -image_data: the flattened pixel vec
/// -width: the image width
/// -height: the image height
/// -channels: N channels (mostly 4)
/// -package_name: pymodule script name
/// -script_path: pymodule parent folder
/// -mppx: x pixel resolution in µm
/// -mppy: y pixel resolution in µm
/// -roi: the roi to select from the pixel array (y0, y1, x0, x1)
/// -outpath: another path that may be used to store additional information (csvs etc)
///
/// Example:
///
///
/// ```
/// # use ndarray::{Array, s, Ix3};
/// # use slideslib::pybridge::run_script;
/// # use ndarray::ShapeBuilder;
/// # use pyo3::{PyErr, prepare_freethreaded_python};
/// # use std::path::PathBuf;
/// # prepare_freethreaded_python();
/// let mut image_data = Array::<u8, Ix3>::ones((50, 50, 4))*255;
/// let mut slice = image_data.slice_mut(s![25.., 25.., 1..2]);
/// slice.fill(0);
/// let res = run_script(&image_data.into_raw_vec(), 50, 50, 4, "count_objects".into(),  "pyfunctions".into(), 1., 1., [0, 50, 0, 50], "/tmp/foo.csv".into(), 
/// PathBuf::from("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_str().unwrap_or("tests/data/02a7b258e875cf073e2421d67ff824cd.tiff").to_string())?;
/// let val = res.output.get(0);
/// assert!(matches!(val, Some(_)), "No fields in count objects output.");
/// // Calculation (sum(obj > 0) / np.prod(objs.shape) * 50 * 50 * 1. * 1. * 64**2)*1e-6 ≈ 2.5,
/// // whereas obj is slightly deformed by binary opening and 64**2 attirbutes to the downsampling
/// // default (c.f. count_objects).
/// assert_eq!((*val.unwrap() - 2.5) < 0.2, true);
///
/// Ok::<(), PyErr>(())
/// ```
pub fn run_script(
    image_data: &Vec<u8>,
    width: u32,
    height: u32,
    channels: u8,
    package_name: String,
    script_path: String,
    mppx: f32,
    mppy: f32,
    roi: [i64; 4],
    outpath: String,
    inpath: String,
) -> PyResult<PythonResponse> {
    Python::with_gil(|py| {
        let data: Vec<u8> = image_data.clone();
        let sys = py.import("sys")?;
        let path = sys.getattr("path")?;

        path.call_method1("append", (script_path,))?;
        let module = PyModule::import(py, package_name.as_str())?;
        let output: (Vec<f32>, Vec<String>) = module
            .getattr("call")?
            .call1((
                data, width, height, channels, mppx, mppy, roi, outpath, inpath,
            ))?
            .extract()?;
        let script_type: String = match module.getattr("TYPE") {
            Ok(val) => val.extract().unwrap_or("Measurement".into()),
            _ => "Measurement".into(),
        };
        Ok(PythonResponse {
            output: output.0,
            field_names: output.1,
            script_type,
        })
    })
}
