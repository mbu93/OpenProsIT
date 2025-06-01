use crate::{
    error::ErrorKind,
    gui_components::Message,
    predictor::{Predictor, PredictorArgs, PreprocessingData, PreprocessingDims},
};
use iced::{advanced::subscription::EventStream, futures::stream::BoxStream};
use ndarray::Array3;
use ndarray::{self, Axis};
use npyz;
use npyz::WriterBuilder;
use std::hash::Hash;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use tch::{nn::ModuleT, CModule, Kind, Tensor};

pub struct DicomPredictor {
    pub image_path: PathBuf,
    pub image_data: Option<Vec<f32>>,
    out_path: String,
    detector: CModule,
    width: u32,
    height: u32,
    pub depth: u32,
}

fn write_array<T, S, D>(writer: impl io::Write, array: &ndarray::ArrayBase<S, D>) -> io::Result<()>
where
    T: Clone + npyz::AutoSerialize,
    S: ndarray::Data<Elem = T>,
    D: ndarray::Dimension,
{
    let shape = array.shape().iter().map(|&x| x as u64).collect::<Vec<_>>();
    let c_order_items = array.iter();

    let mut writer = npyz::WriteOptions::new()
        .default_dtype()
        .shape(&shape)
        .writer(writer)
        .begin_nd()?;
    writer.extend(c_order_items)?;
    writer.finish()
}

impl Predictor for DicomPredictor {
    /// Return the maximum cycle for a progress bar. Equal to the dcm depth.
    ///
    /// Example
    /// ```
    /// # use slideslib::dicom_predictor::DicomPredictor;
    /// # use slideslib::predictor::PredictorArgs;
    /// # use slideslib::predictor::Predictor;
    /// # use slideslib::error::ErrorKind;
    /// # use std::fs;
    /// # use std::path::PathBuf;
    /// let args = PredictorArgs {width: 0, height: 0, depth: 0, path: PathBuf::from("tests/MRI Test")};
    /// let predictor = DicomPredictor::new(args)?;
    /// assert_eq!(predictor.max_progress() as u32, predictor.depth);
    /// Ok::<(), ErrorKind>(())
    fn max_progress(&self) -> usize {
        return self.depth as usize;
    }

    /// Create a new slide predictor instance. Will load the ismil prediction models parts
    /// (backbone and extractor).
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::dicom_predictor::DicomPredictor;
    /// # use slideslib::predictor::PredictorArgs;
    /// # use slideslib::predictor::Predictor;
    /// # use slideslib::error::ErrorKind;
    /// # use std::fs;
    /// # use std::path::PathBuf;
    /// let args = PredictorArgs {width: 0, height: 0, depth: 0, path: PathBuf::from("data").join("preprocessed").join("MRI Test")};
    /// let predictor = DicomPredictor::new(args.clone())?;
    /// fs::rename("models", "models_");
    /// let predictor = DicomPredictor::new(args.clone());
    /// fs::rename("models_", "models");
    /// assert!(predictor.is_err(), "Model displacement not detected!");
    /// Ok::<(), ErrorKind>(())
    /// ```
    fn new(predictor_args: PredictorArgs) -> Result<Self, ErrorKind> {
        let detector = tch::CModule::load("models/mri.pth")
            .map_err(|err| ErrorKind::BackboneLoadError(err.to_string()).into())?;

        return Ok(Self {
            image_path: predictor_args.path.join("whole_inp.npy"),
            image_data: None,
            out_path: String::from(predictor_args.path.as_os_str().to_str().unwrap_or("./")),
            detector,
            width: predictor_args.width,
            height: predictor_args.height,
            depth: predictor_args.depth,
        });
    }

    /// Preprocess the image by reading in the numpy array of histogram matched inputs.
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::dicom_predictor::DicomPredictor;
    /// # use slideslib::error::ErrorKind;
    /// # use slideslib::predictor::PredictorArgs;
    /// # use slideslib::predictor::Predictor;
    /// # use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), slideslib::error::ErrorKind> {
    /// let args = PredictorArgs {width: 0, height: 0, depth: 0, path: PathBuf::from("data").join("preprocessed").join("MRI Test")};
    /// let mut predictor = DicomPredictor::new(args)?;
    /// predictor.preprocess()?;
    /// assert!(predictor.image_data.is_some());
    /// assert!(predictor.image_data.unwrap()[0] + 1.2409786 < 0.001);
    ///
    /// Ok::<(), ErrorKind>(())
    /// # }
    /// ```
    fn preprocess(&mut self) -> Result<Option<PreprocessingData>, ErrorKind> {
        let image_path = self.image_path.clone();
        let bytes = std::fs::read(image_path.clone())
            .map_err(|_| ErrorKind::DicomImageLoadingError(image_path.clone()))?;
        let numpy_data = npyz::NpyFile::new(&bytes[..])
            .map_err(|_| ErrorKind::DicomImageLoadingError(image_path.clone()))?
            .into_vec::<f32>()
            .map_err(|_| ErrorKind::DicomImageLoadingError(image_path.clone()))?;
        numpy_data.to_vec();
        self.image_data = Some(numpy_data);
        Ok(None)
    }

    /// Run the prediction for the new image. A tx is required, as this is supposed to be executed
    /// in a separate thread, due to the possibly long duration of the prediciton procedure.
    /// Furthermore, this function optionally takes values of the preprocessing.
    ///
    /// Example:
    ///
    /// ```
    /// # use slideslib::dicom_predictor::{DicomPredictor};
    /// # use slideslib::error::ErrorKind;
    /// # use std::fs;
    /// # use tch::Tensor;
    /// # use libvips::{VipsImage};
    /// # use openslide_rs::Size;
    /// # use std::path::PathBuf;
    /// # fn main() -> Result<(), slideslib::error::ErrorKind> {
    /// # use std::sync::mpsc::channel;
    /// # use slideslib::predictor::PredictorArgs;
    /// # use slideslib::predictor::Predictor;
    /// # use tch::Tensor;
    ///
    /// let args = PredictorArgs {
    ///     path: PathBuf::from("data").join("preprocessed").join("MRI Test"),
    ///     width: 224*3,
    ///     height: 224,
    ///     depth: 21,
    /// };
    /// let (sender, _) = channel();
    /// let mut predictor = DicomPredictor::new(args)?;
    ///
    /// let (mean, _) = predictor.run(None, None, sender)?;
    /// assert_eq!(mean, Tensor::from(0.0068779210560023785));
    /// Ok::<(), ErrorKind>(())
    /// # }
    /// ```
    fn run(
        &mut self,
        _preprocessed: Option<PreprocessingData>,
        _preprocessing_dims: Option<PreprocessingDims>,
        tx: mpsc::Sender<Message>,
    ) -> Result<(Tensor, Tensor), ErrorKind> {
        let img = match self.image_data.clone() {
            Some(data) => data,
            None => {
                let image_path = self.image_path.clone();
                let bytes = std::fs::read(image_path.clone())
                    .map_err(|_| ErrorKind::DicomImageLoadingError(image_path.clone()))?;
                let numpy_data = npyz::NpyFile::new(&bytes[..])
                    .map_err(|_| ErrorKind::DicomImageLoadingError(image_path.clone()))?
                    .into_vec::<f32>()
                    .map_err(|_| ErrorKind::DicomImageLoadingError(image_path.clone()))?;
                numpy_data.to_vec()
            }
        };

        let tens = Tensor::from_slice(img.as_slice()).view((
            self.height as i64,
            self.width as i64,
            self.depth as i64 + 1,
        ));
        let t1 = tens.narrow(1, 0, 224); // (224, 224, 22)
        let t2 = tens.narrow(1, 224, 224); // (224, 224, 22)
        let t3 = tens.narrow(1, 448, 224); // (224, 224, 22)
        let tens: Tensor = Tensor::stack(&[t1, t2, t3], 0).permute(&[3, 0, 1, 2]);
        let tens = tens;
        let mut outputs = Vec::new();

        for i in 0..tens.size()[0] {
            let input = tens.select(0, i).unsqueeze(0);
            let output = self.detector.forward_t(&input, false).squeeze();
            // outshape [(lesion|segment),(x),(bs),(neurons),(w),(h)][2, 1, 1, 2, 224, 224])
            let extracted = output.select(0, 0).select(0, 1);
            outputs.push(extracted);
            tx.send(Message::UpdateCounter).unwrap_or(());
        }

        let res = tch::Tensor::stack(&outputs, 0);
        let mut out_vec: Vec<f32> = Vec::<f32>::try_from(res.flatten(0, -1)).map_err(|err| {
            ErrorKind::ArrayError(String::from("creating prediction vector"), err.to_string())
        })?;

        out_vec = out_vec
            .iter()
            .map(|v| if *v > 0.5 { 1. } else { 0. })
            .collect();

        let out_arr = Array3::from_shape_vec((22, 224, 224), out_vec)
            .map_err(|err| ErrorKind::ArrayError(String::from("prediction"), err.to_string()))?;
        let out_arr_t =
            ndarray::concatenate(Axis(2), &[out_arr.view(), out_arr.view(), out_arr.view()])
                .map_err(|err| ErrorKind::ArrayError(String::from("prediction"), err.to_string()))?
                .permuted_axes([1, 2, 0]);
        let out_arr_t = out_arr_t;
        let mut file = io::BufWriter::new(
            std::fs::File::create(PathBuf::from(self.out_path.clone()).join("pred.npy"))
                .map_err(|err| ErrorKind::PredWriteError(err.to_string()))?,
        );
        write_array(&mut file, &out_arr_t)
            .map_err(|err| ErrorKind::PredWriteError(err.to_string()))?;
        return Ok((res.mean(Kind::Float), Tensor::new()));
    }
}

// Custom subscription to listen for counter updates
pub struct CounterUpdateSubscription {
    pub receiver: Arc<Mutex<Receiver<Message>>>,
}

impl iced::advanced::subscription::Recipe for CounterUpdateSubscription {
    type Output = Message;

    fn hash(&self, state: &mut iced::advanced::Hasher) {
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: EventStream) -> BoxStream<'static, Self::Output> {
        use iced::futures::stream::StreamExt;
        iced::futures::stream::unfold(self.receiver, |receiver| async move {
            match receiver.lock() {
                Ok(r) => match r.recv() {
                    Ok(message) => Some((message, receiver.clone())),
                    Err(_) => None,
                },
                Err(err) => {
                    println!("Couldn't get receiver lock with error: {:?}", err);
                    None
                }
            }
        })
        .boxed()
    }
}
