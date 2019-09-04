//use crate::config::Config;
use crate::config::{Padding, TextParameters};

use pango;
use pango::prelude::*;
use pango::Layout;
use pango::FontDescription;

use crate::types::maths::{ Rect, Vec2 };

#[derive(Debug)]
pub struct TextRenderer {
    //config: &'a Config,
    font: FontDescription,
    pctx: pango::Context,
    layout: pango::Layout,
}

impl TextRenderer {
    pub fn new(ctx: &cairo::Context, font_name: &str) -> Self {
        let pctx = pangocairo::functions::create_context(ctx)
            .expect("Failed to create pango context.");

        // @IMPORTANT: FontDescription must be freed at some point????
        let font = FontDescription::from_string(font_name);

        // Find font from description -- Arial 10 Bold
        pctx.set_font_description(&font);

        let layout = Layout::new(&pctx);
        layout.set_ellipsize(pango::EllipsizeMode::Middle);

        Self {
            font,
            pctx,
            layout,
        }
    }

    fn current_rect(&self, cursor_pos: &Vec2, padding: &Padding) -> Rect {
        let (width, height) = self.layout.get_pixel_size();
        Rect::new(
            cursor_pos.x, cursor_pos.y,
            width as f64 + (padding.left + padding.right),
            height as f64 + (padding.top + padding.bottom)
        )
    }

    pub fn get_string_rect(&self, parameters: &TextParameters, pos: &Vec2, text: &str) -> Rect {
        self.layout.set_text(text);
        self.layout.set_height(pango::SCALE * parameters.max_height);
        self.layout.set_width(pango::SCALE * parameters.max_width);

        self.current_rect(pos, &parameters.padding)
    }

    pub fn paint_string(&self, ctx: &cairo::Context, parameters: &TextParameters, pos: &Vec2, text: &str) -> Rect {
        self.layout.set_text(text);
        self.layout.set_height(pango::SCALE * parameters.max_height);
        self.layout.set_width(pango::SCALE * parameters.max_width);

        // Move cursor to draw position and draw text.
        ctx.move_to(pos.x + parameters.padding.left, pos.y + parameters.padding.top);
        pangocairo::functions::show_layout(ctx, &self.layout);

        self.current_rect(pos, &parameters.padding)
    }
}
