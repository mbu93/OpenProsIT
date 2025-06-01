use crate::error::ErrorKind;
use crate::gui_components::Message;
use libvips::VipsImage;
use std::path::PathBuf;
use std::sync::mpsc;
use tch::Tensor;

pub struct PreprocessingData {
    pub img: VipsImage,
    pub owidth: i32,
    pub oheight: i32,
    pub nwidth: u32,
    pub nheight: u32,
    pub outdims: openslide_rs::Size,
}

#[derive(Clone, Debug)]
pub struct PreprocessingDims {
    pub owidth: i32,
    pub oheight: i32,
    pub nwidth: u32,
    pub nheight: u32,
    pub outdims: openslide_rs::Size,
}

#[derive(Clone, Debug)]
pub struct PredictorArgs {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub depth: u32,
}

pub trait Predictor {
    fn max_progress(&self) -> usize;
    fn preprocess(&mut self) -> Result<Option<PreprocessingData>, ErrorKind>;

    fn run(
        &mut self,
        preprocessed: Option<PreprocessingData>,
        preprocessing_dims: Option<PreprocessingDims>,
        tx: mpsc::Sender<Message>,
    ) -> Result<(Tensor, Tensor), ErrorKind>;

    fn new(predictor_args: PredictorArgs) -> Result<Self, ErrorKind>
    where
        Self: Sized;
}
