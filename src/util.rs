// STD lib
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// glob
use glob::{glob, Paths};

// local modules
use crate::error::ErrorKind;

/// Write an error to a threadsafe variable or read from it. Can be used in the main GUI, e.g., to
/// communicate errors thrown during script execution to the error modal.
///
/// Example:
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use slideslib::{util::log_or_load_thread_err, error::ErrorKind};
/// let arc = Arc::new(Mutex::new(Some(ErrorKind::ConfigError())));
/// let err = ErrorKind::ConfigError();
/// {
///     let err_ = log_or_load_thread_err(arc.clone(), Some(ErrorKind::ConfigError()));
///     assert!(matches!(err_, None), "Error was not set!");
/// }
/// assert!(
///     matches!(arc.lock().unwrap().clone(), ErrorKind),
///     "Error was not set!"
/// );
/// let err_ = log_or_load_thread_err(arc.clone(), None).ok_or("Error not read!")?;
/// assert!(matches!(err_, ErrorKind), "Error was not read!");
/// Ok::<(), &'static str>(())
/// ```
pub fn log_or_load_thread_err(
    arc: Arc<Mutex<Option<ErrorKind>>>,
    err: Option<ErrorKind>,
) -> Option<ErrorKind> {
    let mut thread_error;
    match (arc.lock(), err) {
        (Ok(val), Some(errval)) => {
            thread_error = val;
            *thread_error = Some(ErrorKind::ThreadError(errval.to_string()));
            return None;
        }
        (Ok(val), None) => {
            thread_error = val;
            let err = thread_error.clone();
            return err;
        }
        _ => {
            println!("Couldn't get thread error logger!");
        }
    };
    None
}

/// Reset an error set to a threadsafe variable. Can be used, e.g., in the main GUI if the modal
/// was closed.
///
/// Example:
/// ```
/// # use std::sync::{Arc, Mutex};
/// # use slideslib::{util::reset_thread_err, error::ErrorKind};
/// let arc = Arc::new(Mutex::new(Some(ErrorKind::ConfigError())));
/// let err = ErrorKind::ConfigError();
/// {   
///     let mut reader = arc.lock().unwrap();
///     *reader = Some(ErrorKind::ConfigError());
///
/// }
/// {
///     let _ = reset_thread_err(&arc);
/// }
/// assert!(matches!(arc.lock().unwrap().clone(), None), "Error was not read!");
/// Ok::<(), &'static str>(())
/// ```
pub fn reset_thread_err(arc: &Arc<Mutex<Option<ErrorKind>>>) {
    let mut thread_error;
    match arc.lock() {
        Ok(val) => {
            thread_error = val;
            *thread_error = None
        }
        _ => {
            println!("Couldn't get thread error logger!");
        }
    };
}

/// Retrieve a file list of currently supported formats (SVS, TIFF) from a folder.
///
/// Example:
///
/// ```
/// # use std::path::PathBuf;
/// # use slideslib::{util::get_file_list, error::ErrorKind};
/// let paths = get_file_list(PathBuf::from("tests").join("data"))?;
/// let mut i = 0;
/// for _ in paths {
///     i += 1;
/// }
/// assert_eq!(i > 0, true);
///
/// Ok::<(), ErrorKind>(())
/// ```
pub fn get_file_list(path: PathBuf) -> Result<std::iter::Chain<Paths, Paths>, ErrorKind> {
    let svs_files = glob(path.join("*.svs").as_os_str().to_str().unwrap_or(""))
        .map_err(|err| ErrorKind::GlobError(path.clone(), err.to_string()).into())?;
    let tiff_files = glob(path.join("*.tiff").as_os_str().to_str().unwrap_or(""))
        .map_err(|err| ErrorKind::GlobError(path.clone(), err.to_string()).into())?;
    let filechain = svs_files.chain(tiff_files);
    return Ok(filechain);
}


