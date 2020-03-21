use serde::Deserialize;

use crate::types::maths::{self, Vec2, Rect};
use crate::config::{Padding, Color, AnchorPosition};
use crate::rendering::window::NotifyWindow;
use image::{FilterType, GenericImageView};
use cairo::ImageSurface;
use crate::rendering::layout::{LayoutBlock, DrawableLayoutElement, Hook};
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct ScrollingTextBlockParameters {
    //pub hook: (AnchorPosition, AnchorPosition),
    //pub offset: Vec2,
    pub padding: Padding,
    pub text: String,
    pub font: String,
    pub color: Color,
    pub max_width: i32,
    pub scroll_speed: f64,

    pub scroll_t: f64,
}

impl DrawableLayoutElement for ScrollingTextBlockParameters {
    fn draw_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        window.context.save();

        let mut text = self.text.clone();
        text = text
            .replace("%s", &window.notification.summary)
            .replace("%b", &window.notification.body);

        // Height of 0 == one line of text.
        window.text.set_text(&text, &self.font, self.max_width, 0);
        let mut rect = window.text.get_rect(&self.padding);

        //let mut pos = self.find_anchor_pos(parent_rect, &rect);
        let mut pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
        pos.x += self.padding.left;
        pos.y += self.padding.top;

        let lerp = maths::lerp(rect.width(), pos.x, self.scroll_t);
        window.context.rectangle(pos.x, pos.y, self.max_width as f64, rect.height());
        window.context.clip();

        pos.x = lerp;

        window.text.paint(&window.context, &pos, &self.color);

        window.context.restore();

        rect.set_x(pos.x - self.padding.left);
        rect.set_y(pos.y - self.padding.top);
        rect
    }

    fn predict_rect_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let mut text = self.text.clone();
        text = text
            .replace("%s", &window.notification.summary)
            .replace("%b", &window.notification.body);

        window.text.set_text(&text, &self.font, self.max_width, 0);
        let mut rect = window.text.get_rect(&self.padding);

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn update(&mut self, delta_time: Duration) -> bool {
        self.scroll_t += self.scroll_speed * delta_time.as_secs_f64();
        self.scroll_t = maths::clamp(self.scroll_t, 0.0, 1.0);

        true
    }
}

