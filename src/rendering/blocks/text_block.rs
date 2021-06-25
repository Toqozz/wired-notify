use serde::Deserialize;

use crate::maths_utility::{Vec2, Rect, MinMax};
use crate::config::{Config, Padding, Color};
use crate::bus::dbus::Notification;
use crate::rendering::{
    window::NotifyWindow,
    layout::{DrawableLayoutElement, LayoutBlock, Hook},
    text::EllipsizeMode,
};
use crate::maths_utility;

#[derive(Debug, Deserialize, Clone)]
pub struct Dimensions {
    width: MinMax,
    height: MinMax,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TextBlockParameters {
    pub padding: Padding,
    //https://developer.gnome.org/pango/stable/pango-Markup.html
    pub text: String,
    pub font: String,
    pub color: Color,
    pub dimensions: Dimensions,

    // -- Optional fields.
    pub color_hovered: Option<Color>,
    pub dimensions_image_hint: Option<Dimensions>,
    pub dimensions_image_app: Option<Dimensions>,
    pub dimensions_image_both: Option<Dimensions>,
    #[serde(default)]
    pub ellipsize: EllipsizeMode,

    // -- Runtime fields
    #[serde(skip)]
    real_text: String,
    #[serde(skip)]
    hover: bool,
}

impl TextBlockParameters {
    fn get_dimensions(&self, notification: &Notification) -> &Dimensions {
        match (notification.app_image.is_some(), notification.hint_image.is_some()) {
            (true, true) => self.dimensions_image_both.as_ref().unwrap_or(&self.dimensions),
            (true, false) => self.dimensions_image_app.as_ref().unwrap_or(&self.dimensions),
            (false, true) => self.dimensions_image_hint.as_ref().unwrap_or(&self.dimensions),
            (false, false) => &self.dimensions,
        }
    }
}

impl DrawableLayoutElement for TextBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        window.context.set_operator(cairo::Operator::Over);

        let dimensions = self.get_dimensions(&window.notification);

        window.text
            .set_text(&self.real_text, &self.font, dimensions.width.max, dimensions.height.max, &self.ellipsize);
        let mut rect =
            window.text.get_sized_padded_rect(&self.padding, dimensions.width.min, dimensions.height.min);

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        let col = if self.hover { self.color_hovered.as_ref().unwrap_or(&self.color) } else { &self.color };
        // Move block to text position (ignoring padding) for draw operation.
        window.text.paint_padded(&window.context, &pos, col, &self.padding);
        // Debug, unpadded drawing, to help users.
        if Config::get().debug {
            let r = window.text.get_sized_rect(dimensions.width.min, dimensions.height.min);
            maths_utility::debug_rect(&window.context, true, pos.x + self.padding.left, pos.y + self.padding.top, r.width(), r.height());
        }

        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let text = maths_utility::format_notification_string(&self.text, &window.notification);

        let dimensions = self.get_dimensions(&window.notification);
        window.text.set_text(&text, &self.font, dimensions.width.max, dimensions.height.max, &self.ellipsize);
        let mut rect = window.text.get_sized_padded_rect(&self.padding, dimensions.width.min, dimensions.height.min);

        self.real_text = text;

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn clicked(&mut self, _window: &NotifyWindow) -> bool {
        maths_utility::find_and_open_url(self.real_text.clone());
        false
    }

    fn hovered(&mut self, entered: bool, _window: &NotifyWindow) -> bool {
        self.hover = entered;
        true
    }
}
