use serde::Deserialize;

use crate::maths_utility::{Vec2, Rect, MinMax};
use crate::config::{Padding, Color, TextDimensionVariants};
use crate::rendering::window::NotifyWindow;
use crate::rendering::layout::{DrawableLayoutElement, LayoutBlock, Hook};

#[derive(Debug, Deserialize, Clone)]
pub struct TextBlockParameters {
    pub padding: Padding,
    //https://developer.gnome.org/pango/stable/pango-Markup.html
    pub text: String,
    pub font: String,
    pub color: Color,
    pub dimensions: TextDimensionVariants,

    #[serde(skip)]
    real_text: String,
}

// @TODO: Some/None for summary/body  We don't want to replace or even add the block if there is no body.
// `rect` will be empty anyway for empty text, but it stops people from putting custom text in those
// blocks, because that will cause it to grow.
impl DrawableLayoutElement for TextBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let dimensions = self.dimensions.get_dimensions(&window.notification);

        window.text.set_text(&self.real_text, &self.font, dimensions.width.max, dimensions.height.max);
        let mut rect = window.text.get_sized_rect(&self.padding, dimensions.width.min, dimensions.height.min);

        let mut pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        // Move block to text position (ignoring padding) for draw operation.
        pos.x += self.padding.left;
        pos.y += self.padding.top;
        window.text.paint(&window.context, &pos, &self.color);
        pos.x -= self.padding.left;
        pos.y -= self.padding.top;

        rect.set_xy(pos.x, pos.y);
        rect
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let mut text = self.text.clone();
        text = text
            .replace("%s", &window.notification.summary)
            .replace("%b", &window.notification.body);

        let dimensions = self.dimensions.get_dimensions(&window.notification);
        window.text.set_text(&text, &self.font, dimensions.width.max, dimensions.height.max);
        let mut rect = window.text.get_sized_rect(&self.padding, dimensions.width.min, dimensions.height.min);

        self.real_text = text;

        let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

        rect.set_xy(pos.x, pos.y);
        rect
    }
}

