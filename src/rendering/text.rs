use crate::config::Config;

use pango;
use pango::prelude::*;
use pango::FontDescription;

use super::maths::Rect;


pub struct TextRenderer<'a> {
    config: &'a Config,

    cairo_context: &'a cairo::Context,
    font: FontDescription,
}

impl<'a> TextRenderer<'a> {
    pub fn new(config: &'a Config, font_name: &str, cairo_context: &'a cairo::Context) -> Self {
        // @IMPORTANT: FontDescription must be freed at soem point.
        let font = FontDescription::from_string(font_name);

        Self {
            config,
            cairo_context,
            font,
        }
    }

    pub fn render_string_pango(&self, x: f64, y: f64, text: &str) -> Rect {
        let pango_context = pangocairo::functions::create_context(&self.cairo_context).unwrap();
        pango_context.set_font_description(&self.font);

        let layout = pango::Layout::new(&pango_context);

        layout.set_text(text);

        let (width, height) = layout.get_pixel_size();

        self.cairo_context.move_to(x, y);

        pangocairo::functions::show_layout(&self.cairo_context, &layout);

        Rect::new(x, y, width as f64, height as f64)
    }
}
