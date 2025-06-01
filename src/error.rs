use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ErrorKind {
    #[error("Couldn't load meta for slide: '{0}'")]
    OpenSlideMetaLoadingError(PathBuf),

    #[error("Couldn't load data of slide: '{0}'")]
    OpenSlideImageLoadingError(PathBuf),
    
    #[error("Couldn't load data of dicom: '{0}'")]
    DicomImageLoadingError(PathBuf),

    #[error("Couldn't read properties of slide: '{0}'")]
    OpenSlidePropertiesError(PathBuf),

    #[error("Couldn't read the mask for slide: '{0} with error {1}")]
    MaskLoadingError(String, String),

    #[error("Couldn't read the mask and image for slide: '{0} with errors {1} {2}")]
    BothLoadingError(String, String, String),

    #[error("The {0} cache couldn't be updated!.")]
    ChacheChangingError(String),

    #[error("VIPS operation failed for {0} with err {1}")]
    VipsOpError(String, String),

    #[error("VIPS operation failed for {0} with err {1}")]
    GlobError(PathBuf, String),

    #[error("Array operation failed for {0} with err {1}")]
    ArrayError(String, String),

    #[error("Python operation failed for {0} with script {1} and err {2}")]
    ScriptError(String, String, String),

    #[error("Fetch error '{0}' at posx: {1} and posy {2} with err {3}")]
    FetchError(String, u32, u32, String),

    #[error("Couldn't read tensor properties, with error '{0}'.")]
    TensorPropError(String),

    #[error("Couldn't iterate prediction or input data, with error '{0}'.")]
    PredIterError(String),
    
    #[error("Couldn't save prediction, with error '{0}'.")]
    PredWriteError(String),

    #[error("Backbone inaccessible with err: {0}.")]
    BackboneLoadError(String),

    #[error("Extractor inaccessible with err: {0}.")]
    ExtractorLoadError(String),

    #[error("Error occured in thread: {0}.")]
    ThreadError(String),

    #[error("Errors occured in thread: {0} {1}.")]
    ThreadMultiError(String, String),

    #[error("Layout crashed rendering with err: {0}.")]
    LayoutError(String),

    #[error("Couldn't read config.json.")]
    ConfigError(),

    #[error("No readable files available.")]
    NoFileError(),
}
