use serde::Deserialize;

use crate::maths_utility::{self, Vec2, Rect};
use crate::config::{Padding};
use crate::rendering::window::NotifyWindow;
use image::FilterType;
use cairo::ImageSurface;
use cairo::Format;
use crate::rendering::layout::{DrawableLayoutElement, LayoutBlock, Hook};

#[derive(Debug, Deserialize, Clone)]
pub enum ImageType {
    App,
    Hint,
}

#[derive(Debug, Deserialize, Clone)]
pub enum FilterMode {
    Nearest,
    Triangle,
    CatmullRom,
    Gaussian,
    Lanczos3,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ImageBlockParameters {
    pub image_type: ImageType,
    // @NOTE: -1 to scale to size with aspect ratio kept?
    pub padding: Padding,
    pub rounding: f64,
    pub scale_width: i32,
    pub scale_height: i32,
    pub min_width: i32,
    pub min_height: i32,
    pub filter_mode: FilterMode,

    // The process of resizing the image and changing colorspace is relatively expensive,
    // so we should cache it.
    #[serde(skip)]
    cached_surface: Option<ImageSurface>,
}

impl DrawableLayoutElement for ImageBlockParameters {
    fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        window.context.set_operator(cairo::Operator::Over);

        // `cached_surface` should always exist on notifications with images, because we always
        // cache it.  If-let is just a precaution here.
        if let Some(ref img_sfc) = self.cached_surface {
            let mut rect = Rect::new(
                0.0, 0.0,
                self.scale_width as f64 + self.padding.width(),
                self.scale_height as f64 + self.padding.height()
            );
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
            rect.set_xy(pos.x, pos.y);

            let (x, y) = (pos.x + self.padding.left, pos.y + self.padding.top);
            window.context.set_source_surface(&img_sfc, x, y);
            maths_utility::cairo_rounded_rectangle(
                &window.context, x, y, self.scale_width as f64, self.scale_height as f64, self.rounding
            );
            //window.context.rectangle(x, y, self.scale_width as f64, self.scale_height as f64);
            window.context.fill();
            maths_utility::debug_rect(&window.context, true, x, y, self.scale_width as f64, self.scale_height as f64);

            rect
        } else {
            let mut rect = Rect::new(0.0, 0.0, 0.0, 0.0);
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
            rect.set_xy(pos.x, pos.y);

            rect
        }
    }

    fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
        let maybe_image =
            match self.image_type {
                ImageType::App => &window.notification.app_image,
                ImageType::Hint => &window.notification.hint_image,
            };

        if let Some(img) = maybe_image {
            let mut rect = Rect::new(
                0.0, 0.0,
                self.scale_width as f64 + self.padding.width(),
                self.scale_height as f64 + self.padding.height(),
            );

            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);

            // Convert our filter_mode to `FilterType`.  We need our own type because `FilterType`
            // is not serializable.
            let filter_type = match self.filter_mode {
                FilterMode::Nearest => FilterType::Nearest,
                FilterMode::Triangle => FilterType::Triangle,
                FilterMode::CatmullRom => FilterType::CatmullRom,
                FilterMode::Gaussian => FilterType::Gaussian,
                FilterMode::Lanczos3 => FilterType::Lanczos3,
            };

            let pixels =
                img.resize_exact(self.scale_width as u32, self.scale_height as u32, filter_type)
                .to_bgra() // Cairo reads pixels back-to-front, so ARgb32 is actually BgrA32.
                .into_raw();

            let stride = cairo::Format::stride_for_width(Format::ARgb32, self.scale_width as u32)
                .expect("Failed to calculate image stride.");

            let image_sfc =
                ImageSurface::create_for_data(pixels, Format::ARgb32, self.scale_width, self.scale_height, stride)
                    .expect("Failed to create image surface.");

            self.cached_surface = Some(image_sfc);

            rect.set_xy(pos.x, pos.y);
            rect
        } else {
            let mut rect = Rect::new(0.0, 0.0, self.min_width as f64, self.min_height as f64);
            let pos = LayoutBlock::find_anchor_pos(hook, offset, parent_rect, &rect);
            rect.set_xy(pos.x, pos.y);
            rect
        }
    }
}
