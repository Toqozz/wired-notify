use serde::Deserialize;

use crate::maths::{Vec2, Rect};
use crate::config::{Padding};
use crate::rendering::window::NotifyWindow;
use image::FilterType;
use cairo::ImageSurface;
use cairo::Format;
use crate::rendering::layout::{DrawableLayoutElement, LayoutBlock, Hook};

#[derive(Debug, Deserialize, Clone)]
pub struct ImageBlockParameters {
    // @NOTE: -1 to scale to size with aspect ratio kept?
    pub padding: Padding,
    pub width: i32,
    pub height: i32,

    // The process of resizing the image and changing colorspace is relatively expensive,
    // so we should cache it.
    #[serde(skip)]
    cached_surface: Option<ImageSurface>,
}

impl DrawableLayoutElement for ImageBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        // `cached_surface` should always exist on notifications with images, because we always
        // cache it.
        if let Some(img_sfc) = &self.cached_surface {
            let mut rect = Rect::new(0.0, 0.0, self.width as f64 + self.padding.width(), self.height as f64 + self.padding.height());
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
            rect.set_xy(pos.x, pos.y);

            let (x, y) = (pos.x + self.padding.left, pos.y + self.padding.top);
            window.context.set_source_surface(&img_sfc, x, y);
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

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        if let Some(img) = &window.notification.image {
            let mut rect = Rect::new(
                0.0,
                0.0,
                self.width as f64 + self.padding.width(),
                self.height as f64 + self.padding.height(),
            );

            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

            // @TODO: config option for scale filter type.
            let pixels =
                img.resize(self.width as u32, self.height as u32, FilterType::Nearest)
                .to_bgra() // Cairo reads pixels back-to-front, so ARgb32 is actually BgrA32.
                .into_raw();

            let stride = cairo::Format::stride_for_width(Format::ARgb32, self.width as u32)
                .expect("Failed to calculate image stride.");

            let image_sfc =
                ImageSurface::create_for_data(pixels, Format::ARgb32, self.width as i32, self.height as i32, stride)
                    .expect("Failed to create image surface.");

            self.cached_surface = Some(image_sfc);

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
