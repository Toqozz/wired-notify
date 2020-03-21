use serde::Deserialize;

use crate::types::maths::{self, Vec2, Rect};
use crate::config::{Padding, Color, AnchorPosition};
use crate::rendering::window::NotifyWindow;
use image::{FilterType, GenericImageView};
use cairo::ImageSurface;
use crate::rendering::layout::{DrawableLayoutElement, LayoutBlock, Hook};

#[derive(Debug, Deserialize, Clone)]
pub struct ImageBlockParameters {
    //pub hook: (AnchorPosition, AnchorPosition),
    // -1 to scale to size with aspect ratio kept?
    //pub offset: Vec2,
    pub padding: Padding,
    pub width: i32,
    pub height: i32,
}

impl DrawableLayoutElement for ImageBlockParameters {
    fn draw_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        if let Some(image) = &window.notification.image {
            let img = image.resize(self.width as u32, self.height as u32, FilterType::Nearest);
            let format = cairo::Format::ARgb32;

            //let (width, height) = img.dimensions();
            let stride = cairo::Format::stride_for_width(format, self.width as u32).expect("Failed to calculate image stride.");
            // Cairo reads pixels back-to-front, so ARgb32 is actually BgrA32.
            let pixels = img.to_bgra().into_raw();
            let image_sfc = ImageSurface::create_for_data(pixels, format, self.width as i32, self.height as i32, stride)
                .expect("Failed to create image surface.");

            let mut rect = Rect::new(0.0, 0.0, self.width as f64 + self.padding.width(), self.height as f64 + self.padding.height());
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
            rect.set_x(pos.x);
            rect.set_y(pos.y);

            let (x, y) = (pos.x + self.padding.left, pos.y + self.padding.top);
            window.context.set_source_surface(&image_sfc, x, y);
            window.context.rectangle(x, y, self.width as f64, self.height as f64);
            window.context.fill();

            rect
        } else {
            let mut rect = Rect::new(0.0, 0.0, 0.0, 0.0);
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
            rect.set_xy(pos.x, pos.y);
            rect
        }
    }

    fn predict_rect_independent(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        if window.notification.image.is_some() {
            let mut rect = Rect::new(
                0.0,
                0.0,
                self.width as f64 + self.padding.width(),
                self.height as f64 + self.padding.height(),
            );

            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

            rect.set_xy(pos.x, pos.y);

            rect
        } else {
            let mut rect = Rect::new(0.0, 0.0, 0.0, 0.0);
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
            rect.set_xy(pos.x, pos.y);
            rect
        }
    }
}
