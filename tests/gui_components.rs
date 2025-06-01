#[cfg(test)]
mod gui_component_tests {
    use std::path::PathBuf;

    use iced::advanced::layout::Limits;
    use iced::advanced::renderer::Style;
    use iced::event::{self, Event, Status};
    use iced::mouse::{self, Cursor};
    use iced::{
        advanced::widget::Tree, advanced::Layout, widget::text, Element, Font, Pixels, Point,
        Rectangle,
    };
    use iced::{Application, Theme, Vector};
    use iced_aw::BOOTSTRAP_FONT_BYTES;
    use iced_tiny_skia::{Backend, Renderer as TRenderer};
    use slideslib::gui_components::base_button;
    use slideslib::{gui_components::*, ImageType, ZoomableImageViewer};
    struct TestApp {
        button_clicked: bool,
        choose_file_param: bool,
        choose_file_clicked: bool,
        choose_script_clicked: bool,
        menu_clicked: bool,
        crop_clicked: bool,
        use_viewer: bool,
        toggle_pred_clicked: bool,
        run_pred_clicked: bool,
        run_script_clicked: bool,
        change_slide_clicked: bool,
        change_slide_param: usize,
        viewer: ZoomableImageViewer,
        view_fn: Box<dyn Fn() -> Element<'static, Message>>, // Store a function pointer or closure
    }

    impl Application for TestApp {
        type Executor = iced::executor::Default;
        type Message = Message;
        type Theme = iced::Theme;
        type Flags = ();

        fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
            let clj = Box::new(|| Element::from(text("")));
            (
                Self {
                    button_clicked: false,
                    choose_file_param: false,
                    menu_clicked: false,
                    choose_file_clicked: false,
                    choose_script_clicked: false,
                    toggle_pred_clicked: false,
                    crop_clicked: false,
                    run_pred_clicked: false,
                    run_script_clicked: false,
                    change_slide_clicked: false,
                    change_slide_param: 100,
                    view_fn: clj,
                    use_viewer: false,
                    viewer: ZoomableImageViewer::new(()).0,
                },
                iced::font::load(BOOTSTRAP_FONT_BYTES).map(|_| Message::Noop),
            )
        }

        fn title(&self) -> String {
            String::from("Test App")
        }

        fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
            match message {
                Message::Noop => {
                    self.button_clicked = true;
                }
                Message::Menu => {
                    self.menu_clicked = true;
                }
                Message::ChooseFile(val) => {
                    self.choose_file_clicked = true;
                    self.choose_file_param = val;
                }
                Message::ChooseScript => {
                    self.choose_script_clicked = true;
                }
                Message::RunScript => {
                    self.run_script_clicked = true;
                }
                Message::RunPrediction(_) => {
                    self.run_pred_clicked = true;
                }
                Message::TogglePred => {
                    self.toggle_pred_clicked = true;
                }
                Message::Crop => {
                    self.crop_clicked = true;
                }
                Message::ChangeFile(val) => {
                    self.change_slide_clicked = true;
                    self.change_slide_param = val
                }
                _ => {}
            }
            iced::Command::none()
        }

        fn view(&self) -> Element<Message> {
            if self.use_viewer {
                return self.viewer.view();
            }
            return (self.view_fn)();
        }
    }

    fn click_component(app: &mut TestApp, cursor: Cursor) -> (Status, Option<Message>) {
        //let mut tree = Tree::empty();
        let mut clipboard = iced::advanced::clipboard::Null {};
        // Simulate the button click event
        let renderer = iced::Renderer::TinySkia(TRenderer::new(
            Backend::new(),
            Font::DEFAULT,
            Pixels::from(1.),
        ));
        let mut view = app.view();
        let mut tree = Tree::new(&view);
        let widget = view.as_widget_mut();
        let node = widget.layout(
            &mut tree,
            &renderer,
            &Limits::new(
                iced::Size {
                    width: 0.,
                    height: 0.,
                },
                iced::Size {
                    width: 800.,
                    height: 600.,
                },
            ),
        );

        let layout = Layout::new(&node);
        let mut vec = Vec::<Message>::new();
        let mut shell = iced::advanced::Shell::new(&mut vec);
        let event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let status = widget.on_event(
            &mut tree,
            event,
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::new(Point::new(0., 0.), iced::Size::new(800., 600.)),
        );

        let event = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        widget.on_event(
            &mut tree,
            event,
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::new(Point::new(0., 0.), iced::Size::new(800., 600.)),
        );
        (status, vec.get(0).cloned())
    }

    fn click_menu(app: &mut TestApp, cursors: Vec<Cursor>) -> (Vec<Status>, Vec<Option<Message>>) {
        let mut status_out: Vec<Status> = Vec::new();
        let mut msg_out: Vec<Option<Message>> = Vec::new();
        //let mut tree = Tree::empty();
        let mut clipboard = iced::advanced::clipboard::Null {};
        // Simulate the button click event
        let mut renderer = iced::Renderer::TinySkia(TRenderer::new(
            Backend::new(),
            Font::DEFAULT,
            Pixels::from(1.),
        ));
        let mut view = app.view();
        let mut tree = Tree::new(&view);
        let widget = view.as_widget_mut();
        let node = widget.layout(
            &mut tree,
            &renderer,
            &Limits::new(
                iced::Size {
                    width: 0.,
                    height: 0.,
                },
                iced::Size {
                    width: 800.,
                    height: 600.,
                },
            ),
        );

        let layout = Layout::new(&node);
        let event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let cursor = *cursors.get(0).unwrap();
        let mut vec = Vec::<Message>::new();
        let mut shell = iced::advanced::Shell::new(&mut vec);
        let status = widget.on_event(
            &mut tree,
            event,
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::new(Point::new(0., 0.), iced::Size::new(800., 600.)),
        );

        let event = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        widget.on_event(
            &mut tree,
            event,
            layout,
            cursor,
            &renderer,
            &mut clipboard,
            &mut shell,
            &Rectangle::new(Point::new(0., 0.), iced::Size::new(800., 600.)),
        );
        let theme = Theme::Light;

        widget.draw(
            &tree,
            &mut renderer,
            &theme,
            &Style::default(),
            layout,
            cursor,
            &Rectangle::new(Point::new(0., 0.), iced::Size::new(800., 600.)),
        );
        match widget.overlay(&mut tree, layout, &renderer, Vector::new(0., 0.)) {
            Some(mut overlay) => {
                let layout_ = overlay.layout(
                    &renderer,
                    iced::Size {
                        width: 100.,
                        height: 100.,
                    },
                );
                let layout_ = Layout::new(&layout_);
                overlay.draw(&mut renderer, &theme, &Style::default(), layout_, cursor);
                for cursor in &cursors[1..] {
                    let mut vec = Vec::<Message>::new();
                    let mut shell = iced::advanced::Shell::new(&mut vec);
                    let mut event = Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
                    overlay.on_event(
                        event,
                        layout_,
                        *cursor,
                        &renderer,
                        &mut clipboard,
                        &mut shell,
                    );
                    event = Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
                    overlay.on_event(
                        event,
                        layout_,
                        *cursor,
                        &renderer,
                        &mut clipboard,
                        &mut shell,
                    );
                    status_out.push(status);
                    msg_out.push(vec.get(0).cloned());
                }
            }
            None => {}
        };
        return (status_out, msg_out);
    }

    #[test]
    fn buttons_work() {
        let (mut app, _) = TestApp::new(());
        app.view_fn = Box::new(|| {
            let cmp = base_button(text(""), Some(Message::Noop));
            Element::from(cmp)
        });
        let cursor = Cursor::Available(Point::new(0.0, 0.0));
        let (status, msg) = click_component(&mut app, cursor);
        // Check if Button fires event
        assert_eq!(status, event::Status::Captured);

        // Check if message was thrown

        // Check the Message triggers the right action
        assert!(matches!(msg, Some(_)), "The message was not thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(&app.button_clicked, &true);

        // Check if action is suppressed when button is disabled
        app.button_clicked = false;
        app.view_fn = Box::new(|| {
            let cmp = base_button(text(""), None);
            Element::from(cmp)
        });
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, None), "The message was not suppresed!");
    }

    #[test]
    fn labeled_buttons_work() {
        let (mut app, _) = TestApp::new(());
        app.view_fn = Box::new(|| {
            let cmp = labeled_button("", Some(Message::Noop));
            Element::from(cmp)
        });
        let cursor = Cursor::Available(Point::new(0.0, 0.0));
        let (status, msg) = click_component(&mut app, cursor);
        // Check if Button fires event
        assert_eq!(status, event::Status::Captured);

        // Check if message was thrown

        // Check the Message triggers the right action
        assert!(matches!(msg, Some(_)), "The message was not thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(&app.button_clicked, &true);

        // Check if action is suppressed when button is disabled
        app.button_clicked = false;
        app.view_fn = Box::new(|| {
            let cmp = labeled_button("", None);
            Element::from(cmp)
        });
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, None), "The message was not suppresed!");
    }

    #[test]
    fn labeled_list_buttons_work() {
        let (mut app, _) = TestApp::new(());
        app.view_fn = Box::new(|| {
            let cmp = labeled_list_button("", Some(Message::Noop));
            Element::from(cmp)
        });
        let cursor = Cursor::Available(Point::new(0.0, 0.0));
        let (status, msg) = click_component(&mut app, cursor);
        // Check if Button fires event
        assert_eq!(status, event::Status::Captured);

        // Check if message was thrown

        // Check the Message triggers the right action
        assert!(matches!(msg, Some(_)), "The message was not thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(&app.button_clicked, &true);

        // Check if action is suppressed when button is disabled
        app.button_clicked = false;
        app.view_fn = Box::new(|| {
            let cmp = labeled_list_button("", None);
            Element::from(cmp)
        });
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, None), "The message was not suppresed!");
    }

    #[test]
    fn menu_works() {
        let (mut app, _) = TestApp::new(());
        app.view_fn = Box::new(|| {
            let menu = default_menu();
            Element::from(menu)
        });
        let cursors = [
            Cursor::Available(Point::new(10.0, 10.0)),
            Cursor::Available(Point::new(10.0, 50.0)),
            Cursor::Available(Point::new(10.0, 70.0)),
            Cursor::Available(Point::new(10.0, 90.0)),
        ];
        let (status_arr, msg_arr) = click_menu(&mut app, Vec::from(cursors));
        for status in status_arr {
            assert_eq!(status, event::Status::Captured);
        }
        for msg in msg_arr {
            assert!(matches!(msg, Some(_)), "The message was thrown!");
            let _ = app.update(msg.unwrap());
        }
        assert_eq!(app.choose_file_clicked, true);
        assert_eq!(app.choose_script_clicked, true);
        assert_eq!(app.choose_file_param, false);
    }

    #[test]
    fn gui_works() {
        let (mut app, _) = TestApp::new(());
        app.use_viewer = true;
        app.viewer.image_path = Vec::from([PathBuf::from("somename.svs")]);

        // Test if menu can be expanded
        let cursor = Cursor::Available(Point::new(10., 5.0));
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, Some(_)), "The message was thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(app.menu_clicked, true);

        // Test if crop is clickable
        app.viewer.imagetype = ImageType::WSI;
        let cursor = Cursor::Available(Point::new(80., 5.0));
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, Some(_)), "The message was thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(app.crop_clicked, true);

        // Test if crop is disabled for MRI
        app.viewer.imagetype = ImageType::DICOM;
        let cursor = Cursor::Available(Point::new(80., 5.0));
        let (_, msg) = click_component(&mut app, cursor);
        assert!(
            matches!(msg, None),
            "The message was (expectedly) not thrown!"
        );

        // Test if run script is clickable
        let cursor = Cursor::Available(Point::new(160., 5.0));
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, Some(_)), "The message was thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(app.run_script_clicked, true);

        // Test if run prediction is clickable
        let cursor = Cursor::Available(Point::new(240., 5.0));
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, Some(_)), "The message was thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(app.run_pred_clicked, true);

        // Test if slide can be change_slide
        let cursor = Cursor::Available(Point::new(10., 45.0));
        let (_, msg) = click_component(&mut app, cursor);
        assert!(matches!(msg, Some(_)), "The message was thrown!");
        let _ = app.update(msg.unwrap());
        assert_eq!(app.change_slide_clicked, true);
        assert_eq!(app.change_slide_param, 0);
    }
}
