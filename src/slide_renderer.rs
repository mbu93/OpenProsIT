use iced::advanced::image::Renderer as ImageRenderer;
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::Style;
use iced::advanced::widget::{self, Tree, Widget};
use iced::widget::image::Handle as ImageHandle;

use iced::{mouse, Element, Length, Rectangle, Size};

use ndarray::ArrayView;
use ndarray::{s, Array};

use crate::error::ErrorKind;

use crate::renderer::{draw_rect, BaseView, BaseViewArgs};

pub struct SlideView {
    pub view: BaseView,
}

impl SlideView {
    pub fn new(args: BaseViewArgs) -> Self {
        Self {
            view: BaseView::new(args),
        }
    }
}
impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for SlideView
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
            let cache;
            cache = self.view.cache.borrow();
            let width = self.view.cache_size.w as usize;
            let height = self.view.cache_size.h as usize;
            let channels = cache.len() as usize / height / width;
            let array = match ArrayView::from_shape((height, width, channels), &cache) {
                Ok(val) => val,
                Err(err) => {
                    println!(
                        "{}",
                        ErrorKind::ArrayError(String::from("Renderer"), err.to_string())
                            .to_string()
                    );
                    return;
                }
            };
            let position_details = self.view.get_position_details();
            let bounds = position_details.bounds;
            let width = position_details.width;
            let height = position_details.height;
            let hmax = position_details.hmax;
            let wmax = position_details.wmax;
            let yoffset = position_details.yoffset;
            let xoffset = position_details.xoffset;

            let mut flat_vec = Array::ones((height, width, 4)) * 255;
            flat_vec
                .slice_mut(s![.., .., 0..channels])
                .assign(&array.slice(s!(
                    bounds.y as usize..bounds.y as usize + height,
                    bounds.x as usize..bounds.x as usize + width,
                    0..channels
                )));

            flat_vec = flat_vec.into_owned();
            let sel_bounds = self.view.get_selection_bounds();
            if let Some(sel_bounds) = sel_bounds {
                draw_rect(&mut flat_vec, sel_bounds, None);
            }

            let flat_vec = flat_vec.into_owned().into_raw_vec();
            let image_handle = ImageHandle::from_pixels(width as u32, height as u32, flat_vec);
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

impl<'a, Message, Theme, Renderer> From<SlideView> for Element<'a, Message, Theme, Renderer>
where
    Renderer: ImageRenderer<Handle = ImageHandle>,
{
    fn from(slideview: SlideView) -> Self {
        Self::new(slideview)
    }
}
