use iced::advanced::image::Renderer as ImageRenderer;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::Style;
use iced::advanced::widget::{self, Tree, Widget};
use iced::widget::image::Handle as ImageHandle;

use iced::{mouse, Element, Length, Rectangle, Size};

use ndarray::{s, Array3};
use ndarray::{Array4, ArrayView};

use crate::error::ErrorKind;

use crate::renderer::{BaseView, BaseViewArgs};

fn normalize_slice(mut slice: ndarray::ArrayViewMut3<f32>) {
    // Manually calculate the min and max
    let mut min = f32::MAX;
    let mut max = f32::MIN;

    for &value in slice.iter() {
        if value < min {
            min = value;
        }
        if value > max {
            max = value;
        }
    }

    // Normalize slice values to [0, 1] range, only if min != max
    if max > min {
        slice.mapv_inplace(|x| (x - min) / (max - min));
    }
}

fn convert_to_rgba(array: Array3<u8>) -> Array4<u8> {
    // We are creating a new 4-channel RGBA array.
    let (height, width, depth) = array.dim();
    let mut rgba_array = Array4::<u8>::zeros((height, width, depth, 4)); // 4 channels for RGBA
    rgba_array
        .slice_mut(s![.., .., .., 0])
        .assign(&array.slice(s![.., .., ..]));
    rgba_array
        .slice_mut(s![.., .., .., 1])
        .assign(&array.slice(s![.., .., ..]));
    rgba_array
        .slice_mut(s![.., .., .., 2])
        .assign(&array.slice(s![.., .., ..]));
    rgba_array.slice_mut(s![.., .., .., 3]).fill(255);

    rgba_array
}
pub struct DicomView {
    pub view: BaseView,
    current_pos: usize,
}

impl DicomView {
    pub fn new(args: BaseViewArgs, current_pos: usize) -> Self {
        Self {
            view: BaseView::new(args),
            current_pos,
        }
    }
}
impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for DicomView
where
    Renderer: ImageRenderer<Handle = ImageHandle>,
{
    fn size(&self) -> Size<Length> {
        return Size::new(Length::Fill, Length::Fill);
    }
    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(
            self.view.viewport_size.w as f32,
            self.view.viewport_size.h as f32,
        ))
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &Style,
        _layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        {
            let input_data = &self.view.cache.borrow();
            let width = self.view.cache_size.w as usize;
            let height = self.view.cache_size.h as usize;
            let depth = input_data.len() / (width * height * 4);

            let mut pred_data_u8: Array3<u8> = Array3::zeros((height, width, depth));
            if self.view.mask_active {
                let c = &self.view.mask_cache.borrow();

                let casted: Vec<f32> = c
                    .chunks_exact(4) // Create chunks of 4 bytes
                    .map(|chunk| f32::from_ne_bytes(chunk.try_into().unwrap())) // Convert each chunk into f32
                    .collect();

                let array_ = match ArrayView::from_shape((height, width, depth), &casted) {
                    Ok(val) => val.to_owned(),
                    Err(err) => {
                        println!(
                            "{}",
                            ErrorKind::ArrayError(String::from("Renderer"), err.to_string())
                                .to_string()
                        );
                        return;
                    }
                };
                pred_data_u8 = array_.mapv(|x| (x * 255.0) as u8);
            }

            let casted: Vec<f32> = input_data
                .chunks_exact(4) // Create chunks of 4 bytes
                .map(|chunk| f32::from_ne_bytes(chunk.try_into().unwrap())) // Convert each chunk into f32
                .collect();
            let mut array_ = match ArrayView::from_shape((height, width, depth), &casted) {
                Ok(val) => val.to_owned(),
                Err(err) => {
                    println!(
                        "{}",
                        ErrorKind::ArrayError(String::from("Renderer"), err.to_string())
                            .to_string()
                    );
                    return;
                }
            };
            // Normalize slices between 0 and 1: [:, :224, :], [:, 224..448, :], [:, 448.., :]
            //normalize_slice(array_.slice_mut(s![.., ..height, ..])); // First region
            //normalize_slice(array_.slice_mut(s![.., height..height * 2, ..])); // Second region
            //normalize_slice(array_.slice_mut(s![.., height * 2.., ..])); // Third region

            normalize_slice(array_.slice_mut(s![.., ..224, ..])); // First region
            normalize_slice(array_.slice_mut(s![.., 224..448, ..])); // Second region
            normalize_slice(array_.slice_mut(s![.., 448.., ..])); // Third region

            // Parse the array as u8 for RGB rendering
            let array_u8 = array_.mapv(|x| (x * 255.0) as u8);

            // Get geometric information for rendering
            let position_details = self.view.get_position_details();
            let hmax = position_details.hmax;
            let wmax = position_details.wmax;
            let yoffset = position_details.yoffset;
            let xoffset = position_details.xoffset;
            // Convert data from gray to RGB
            let mut array_rgb = convert_to_rgba(array_u8);

            if self.view.mask_active {
                array_rgb.slice_mut(s![.., .., .., 0]).zip_mut_with(
                    &pred_data_u8,
                    |a_val, &b_val| {
                        *a_val = (*a_val as f32 * 0.25) as u8 + (b_val as f32 * 0.75) as u8;
                    },
                );
            }

            let array = array_rgb.slice(s![.., .., self.current_pos, ..]);
            // Create a vector to be read by the renderer
            let flat_vec = array.into_owned().into_owned().into_raw_vec();
            let image_handle =
                ImageHandle::from_pixels(width as u32, height as u32, flat_vec.clone());

            // Render everything
            renderer.draw(
                image_handle,
                iced::widget::image::FilterMethod::Linear,
                iced::Rectangle {
                    x: xoffset,
                    y: yoffset,
                    width: wmax - 5.,
                    height: hmax - 5.,
                },
            );
        }
    }
}

impl<'a, Message, Theme, Renderer> From<DicomView> for Element<'a, Message, Theme, Renderer>
where
    Renderer: ImageRenderer<Handle = ImageHandle>,
{
    fn from(dicomview: DicomView) -> Self {
        Self::new(dicomview)
    }
}
