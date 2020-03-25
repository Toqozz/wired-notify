use serde::Deserialize;

use crate::types::maths::{self, Vec2, Rect};
use crate::config::{Padding, Color, AnchorPosition};
use crate::rendering::window::NotifyWindow;
use image::{FilterType, GenericImageView};
use cairo::ImageSurface;

use crate::rendering::layout::{DrawableLayoutElement, Hook};
use crate::rendering::layout::LayoutBlock;
use std::time::Duration;

#[derive(Debug, Deserialize, Clone)]
pub struct NotificationBlockParameters {
    pub monitor: i32,
    //pub monitor_hook: (AnchorPosition, AnchorPosition),
    //pub monitor_offset: Vec2,

    pub border_width: f64,
    pub background_color: Color,
    pub border_color: Color,

    pub gap: Vec2,
    pub notification_hook: AnchorPosition,
}

impl DrawableLayoutElement for NotificationBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        // Clear
        window.context.set_operator(cairo::Operator::Clear);
        window.context.paint();

        window.context.set_operator(cairo::Operator::Source);

        // Draw border + background.
        let bd_color = &self.border_color;
        window.context.set_source_rgba(bd_color.r, bd_color.g, bd_color.b, bd_color.a);
        window.context.paint();

        let bg_color = &self.background_color;
        let bw = &self.border_width;
        window.context.set_source_rgba(bg_color.r, bg_color.g, bg_color.b, bg_color.a);
        window.context.rectangle(
            *bw, *bw,     // x, y
            parent_rect.width() - bw * 2.0, parent_rect.height() - bw * 2.0,
        );
        window.context.fill();

        // Base notification background doesn't actually take up space, so use same rect.
        parent_rect.clone()
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        parent_rect.clone()
    }
}

