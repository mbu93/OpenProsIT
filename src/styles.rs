use iced::border::Radius;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme};
use iced_style::progress_bar;

pub struct TopbarStyle;
impl container::StyleSheet for TopbarStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let plt = iced::Theme::Dark.extended_palette();
        container::Appearance {
            background: Some(plt.primary.base.color.into()),
            text_color: Some(Color::BLACK),
            ..Default::default()
        }
    }
}

pub struct ButtonStyle;
impl button::StyleSheet for ButtonStyle {
    type Style = iced::Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            text_color: Color::BLACK,
            border: Border::with_radius(0.0),
            background: Some(Color::TRANSPARENT.into()),
            ..Default::default()
        }
    }

    fn hovered(&self, _style: &Self::Style) -> button::Appearance {
        let plt = iced::Theme::Dark.extended_palette();

        button::Appearance {
            background: Some(plt.primary.base.color.into()),
            border: Border::with_radius(0.0),
            text_color: Color::WHITE,
            ..Default::default()
        }
    }
}

pub struct ProgressStyle(pub(super) Color);

impl progress_bar::StyleSheet for ProgressStyle {
    type Style = iced_style::Theme;

    fn appearance(&self, _style: &Self::Style) -> progress_bar::Appearance {
        progress_bar::Appearance {
            background: Background::Color(Color::from_rgb(0.0, 0.0, 0.0)),
            bar: Background::Color(self.0),
            border_radius: Radius::from(0.),
        }
    }
}
