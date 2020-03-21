use serde::Deserialize;

use crate::types::maths::{self, Vec2, Rect};
use crate::config::{Padding, Color, AnchorPosition};
use crate::rendering::window::NotifyWindow;
use image::{FilterType, GenericImageView};
use cairo::ImageSurface;
use crate::rendering::layout::{DrawableLayoutElement, LayoutBlock, Hook};

#[derive(Debug, Deserialize, Clone)]
pub struct TextBlockParameters {
    //pub hook: (AnchorPosition, AnchorPosition),
    //pub offset: Vec2,
    pub padding: Padding,
    //https://developer.gnome.org/pango/stable/pango-Markup.html
    pub text: String,
    pub font: String,
    pub color: Color,
    pub max_width: i32,
    pub max_height: i32,
}

impl DrawableLayoutElement for TextBlockParameters {
    fn draw_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        // TODO: Some/None for summary/body?  We don't want to replace or even add the block if there is no body.
        let mut text = self.text.clone();
        text = text
            .replace("%s", &window.notification.summary)
            .replace("%b", &window.notification.body);

        window.text.set_text(&text, &self.font, self.max_width, self.max_height);
        let mut rect = window.text.get_rect(&self.padding);

        let mut pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
        pos.x += self.padding.left;
        pos.y += self.padding.top;

        window.text.paint(&window.context, &pos, &self.color);

        rect.set_x(pos.x - self.padding.left);
        rect.set_y(pos.y - self.padding.top);
        rect
    }

    fn predict_rect_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let mut text = self.text.clone();
        text = text
            .replace("%s", &window.notification.summary)
            .replace("%b", &window.notification.body);

        window.text.set_text(&text, &self.font, self.max_width, self.max_height);
        let mut rect = window.text.get_rect(&self.padding);

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        rect.set_xy(pos.x, pos.y);
        rect
    }
}

