use crate::predictor::PreprocessingDims;
use crate::styles::ButtonStyle;

use iced::advanced::layout::{self, Layout, Node};
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::advanced::{self, Clipboard, Shell};
use iced::alignment::Alignment;
use iced::event;
use iced::keyboard::Key;
use iced::widget::{button, column, container, text, Container, scrollable};
use iced::{alignment, Color, Element, Event, Length, Point, Rectangle, Shadow, Size, Vector};
use iced::{mouse, theme};
use iced_aw::menu::{Item, Menu, MenuBar, StyleSheet};
use iced_aw::style::MenuBarStyle;
use iced_aw::{menu, menu_bar, menu_items};

#[derive(Debug, Clone)]
pub enum Message {
    RunScript,
    ChooseScript,
    Menu,
    DragStart,
    DragEnd,
    NoEvent,
    ChangeFile(usize),
    MouseMove(Point),
    KeyPressed(Key),
    OnVerResize(u16),
    OnHorResize(u16),
    ChooseFile(bool),
    RunPrediction(Option<PreprocessingDims>),
    TogglePred,
    UpdateCounter,
    Crop,
    WindowResized((u32, u32)),
    HideModal,
    Noop,
}

/// A button container that emits a message upon click. Disabled if no Message is provided.
pub fn base_button<'a>(
    content: impl Into<Element<'a, Message, iced::Theme, iced::Renderer>>,
    msg: Option<Message>,
) -> button::Button<'a, Message, iced::Theme, iced::Renderer> {
    let button_ = button(content)
        .padding([2, 4])
        .height(40)
        .style(iced::theme::Button::Custom(Box::new(ButtonStyle {})));

    return match msg {
        Some(sig) => button_.on_press(sig),
        None => button_,
    };
}

/// A button with a text that emits a message upon click. Disabled if no Message is provided.
pub fn labeled_button<'a>(
    label: &str,
    msg: Option<Message>,
) -> button::Button<'a, Message, iced::Theme, iced::Renderer> {
    base_button(
        text(label)
            .size(12.)
            .width(Length::Fill)
            .height(Length::Fill)
            .vertical_alignment(alignment::Vertical::Center)
            .horizontal_alignment(alignment::Horizontal::Center),
        msg,
    )
}

/// A button with a text that emits a message upon click. Disabled if no Message is provided. Can
/// be used in a menu.
pub fn labeled_list_button<'a>(
    label: &str,
    msg: Option<Message>,
) -> button::Button<'a, Message, iced::Theme, iced::Renderer> {
    base_button(
        text(label)
            .size(12.)
            .width(Length::Fill)
            .vertical_alignment(alignment::Vertical::Center)
            .horizontal_alignment(alignment::Horizontal::Left),
        msg,
    )
    .height(20)
}

/// A container for a menu. Contains 3 buttons for choosing file, folder, or script.
pub fn default_menu<'a>() -> MenuBar<'a, Message, iced::Theme, iced::Renderer> {
    let file_select = "Choose File";
    let folder_select = "Choose Folder";
    let script_select = "Set Script";
    let menu = "Menu";

    menu_bar!((labeled_button(menu, Some(Message::Menu)), {
        let sub1 = Menu::new(menu_items!((labeled_list_button(
            script_select,
            Some(Message::ChooseScript)
        ))(labeled_list_button(
            file_select,
            Some(Message::ChooseFile(true))
        ))(labeled_list_button(
            folder_select,
            Some(Message::ChooseFile(false))
        ))))
        .width(150);
        sub1
    }))
    .width(75.)
    .height(40.)
    .spacing(4.)
    .padding(5.)
    .draw_path(menu::DrawPath::Backdrop)
    .style(|theme: &iced::Theme| menu::Appearance {
        path_border: iced::Border {
            radius: [0.0; 4].into(),
            ..Default::default()
        },
        bar_background: iced::Background::Color(Color::TRANSPARENT),
        menu_background: iced::Background::Color(Color::WHITE),
        menu_shadow: Shadow {
            color: Color::from_rgb(0., 0., 0.),
            offset: Vector::new(0., 0.),
            blur_radius: 5.,
        },
        ..theme.appearance(&MenuBarStyle::Default)
    })
}

// Credit to https://github.com/iced-rs/iced/blob/0.12.0/examples/modal/src/main.rs
// for implementing the code below
pub struct Modal<'a, Message, Theme, Renderer> {
    base: Element<'a, Message, Theme, Renderer>,
    modal: Element<'a, Message, Theme, Renderer>,
    on_blur: Option<Message>,
}

impl<'a, Message, Theme, Renderer> Modal<'a, Message, Theme, Renderer> {
    /// Returns a new [`Modal`]
    pub fn new(
        base: impl Into<Element<'a, Message, Theme, Renderer>>,
        modal: impl Into<Element<'a, Message, Theme, Renderer>>,
    ) -> Self {
        Self {
            base: base.into(),
            modal: modal.into(),
            on_blur: None,
        }
    }

    /// Sets the message that will be produces when the background
    /// of the [`Modal`] is pressed
    pub fn on_blur(self, on_blur: Message) -> Self {
        Self {
            on_blur: Some(on_blur),
            ..self
        }
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Modal<'a, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
    Message: Clone,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.base),
            widget::Tree::new(&self.modal),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[&self.base, &self.modal]);
    }

    fn size(&self) -> Size<Length> {
        self.base.as_widget().size()
    }

    fn layout(
        &self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.base
            .as_widget()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn on_event(
        &mut self,
        state: &mut widget::Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.base.as_widget_mut().on_event(
            &mut state.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn draw(
        &self,
        state: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.base.as_widget().draw(
            &state.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        state: &'b mut widget::Tree,
        layout: Layout<'_>,
        _renderer: &Renderer,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        Some(overlay::Element::new(Box::new(Overlay {
            position: layout.position() + translation,
            content: &mut self.modal,
            tree: &mut state.children[1],
            size: layout.bounds().size(),
            on_blur: self.on_blur.clone(),
        })))
    }

    fn mouse_interaction(
        &self,
        state: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.base.as_widget().mouse_interaction(
            &state.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &self,
        state: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.base
            .as_widget()
            .operate(&mut state.children[0], layout, renderer, operation);
    }
}

pub fn modal(err: String) -> Container<'static, Message> {
    container(
        column![
            text("Error occured").size(18),
            scrollable(text(err).size(11)).direction(scrollable::Direction::Both {
                vertical: scrollable::Properties::new(),
                horizontal:  scrollable::Properties::new()
            }).width(280).height(140),
            button(text("Ok")).on_press(Message::HideModal),
        ]
        .spacing(20)
        .align_items(alignment::Horizontal::Center.into()),
    )
    .width(300)
    .padding(10)
    .center_x()
    .center_y()
    .style(theme::Container::Box)
}

struct Overlay<'a, 'b, Message, Theme, Renderer> {
    position: Point,
    content: &'b mut Element<'a, Message, Theme, Renderer>,
    tree: &'b mut widget::Tree,
    size: Size,
    on_blur: Option<Message>,
}

impl<'a, 'b, Message, Theme, Renderer> overlay::Overlay<Message, Theme, Renderer>
    for Overlay<'a, 'b, Message, Theme, Renderer>
where
    Renderer: advanced::Renderer,
    Message: Clone,
{
    fn layout(&mut self, renderer: &Renderer, _bounds: Size) -> layout::Node {
        let limits = layout::Limits::new(Size::ZERO, self.size)
            .width(Length::Fill)
            .height(Length::Fill);

        let child = self
            .content
            .as_widget()
            .layout(self.tree, renderer, &limits)
            .align(Alignment::Center, Alignment::Center, limits.max());

        layout::Node::with_children(self.size, vec![child]).move_to(self.position)
    }

    fn on_event(
        &mut self,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
    ) -> event::Status {
        let content_bounds = layout
            .children()
            .next()
            .unwrap_or(Layout::new(&Node::new(iced::Size::new(0., 0.))))
            .bounds();

        if let Some(message) = self.on_blur.as_ref() {
            if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = &event {
                if !cursor.is_over(content_bounds) {
                    shell.publish(message.clone());
                    return event::Status::Captured;
                }
            }
        }

        self.content.as_widget_mut().on_event(
            self.tree,
            event,
            layout
                .children()
                .next()
                .unwrap_or(Layout::new(&Node::new(iced::Size::new(0., 0.)))),
            cursor,
            renderer,
            clipboard,
            shell,
            &layout.bounds(),
        )
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                ..renderer::Quad::default()
            },
            Color {
                a: 0.80,
                ..Color::BLACK
            },
        );

        self.content.as_widget().draw(
            self.tree,
            renderer,
            theme,
            style,
            layout
                .children()
                .next()
                .unwrap_or(Layout::new(&Node::new(iced::Size::new(0., 0.)))),
            cursor,
            &layout.bounds(),
        );
    }

    fn operate(
        &mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation<Message>,
    ) {
        self.content.as_widget().operate(
            self.tree,
            layout
                .children()
                .next()
                .unwrap_or(Layout::new(&Node::new(iced::Size::new(0., 0.)))),
            renderer,
            operation,
        );
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            self.tree,
            layout
                .children()
                .next()
                .unwrap_or(Layout::new(&Node::new(iced::Size::new(0., 0.)))),
            cursor,
            viewport,
            renderer,
        )
    }

    fn overlay<'c>(
        &'c mut self,
        layout: Layout<'_>,
        renderer: &Renderer,
    ) -> Option<overlay::Element<'c, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            self.tree,
            layout
                .children()
                .next()
                .unwrap_or(Layout::new(&Node::new(iced::Size::new(0., 0.)))),
            renderer,
            Vector::ZERO,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<Modal<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Theme: 'a,
    Message: 'a + Clone,
    Renderer: 'a + advanced::Renderer,
{
    fn from(modal: Modal<'a, Message, Theme, Renderer>) -> Self {
        Element::new(modal)
    }
}
