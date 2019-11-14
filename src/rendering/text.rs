//use crate::config::Config;
use crate::config::{Padding, Color};

use pango;
use pango::prelude::*;
use pango::Layout;
use pango::FontDescription;

use crate::types::maths::{ Rect, Vec2 };

#[derive(Debug)]
pub struct TextRenderer {
    //config: &'a Config,
    pctx: pango::Context,
    layout: pango::Layout,
}

impl TextRenderer {
    pub fn new(ctx: &cairo::Context) -> Self {
        let pctx = pangocairo::functions::create_context(ctx)
            .expect("Failed to create pango context.");

        let layout = Layout::new(&pctx);
        // TODO: this should be a config option.
        layout.set_ellipsize(pango::EllipsizeMode::Middle);

        Self {
            pctx,
            layout,
        }
    }

    fn current_rect(&self, cursor_pos: &Vec2, padding: &Padding, vc: bool) -> Rect {
        let (width, height) = self.layout.get_pixel_size();
        let mut pos = cursor_pos.clone();
        let mut offset = 0.0;
        if vc {
            offset = height as f64 * 0.5;
        }

        Rect::new(
            pos.x - padding.left, pos.y - padding.top - offset,
            width as f64 + (padding.left + padding.right),
            height as f64 + (padding.top + padding.bottom)
        )
    }

    pub fn get_string_rect(
        &self,
        text: &str,
        pos: &Vec2,
        padding: &Padding,
        font: &str,
        vertical_center: bool,
        max_width: i32,
        max_height: i32) -> Rect {

        let font_dsc = FontDescription::from_string(font);
        self.pctx.set_font_description(&font_dsc);

        self.layout.set_markup(text);
        //self.layout.set_text(text);
        self.layout.set_height(pango::SCALE * max_height);
        self.layout.set_width(pango::SCALE * max_width);

        self.current_rect(pos, padding, vertical_center)
    }

    pub fn paint_string(
        &self,
        ctx: &cairo::Context,
        text: &str,
        pos: &Vec2,
        padding: &Padding,
        font: &str,
        color: &Color,
        vertical_center: bool,
        max_width: i32,
        max_height: i32) -> Rect {

        let font_dsc = FontDescription::from_string(font);
        self.pctx.set_font_description(&font_dsc);

        self.layout.set_markup(text);
        //self.layout.set_text(text);
        self.layout.set_height(pango::SCALE * max_height);
        self.layout.set_width(pango::SCALE * max_width);

        let mut offset = 0.0;
        if vertical_center {
            let (_, height) = self.layout.get_pixel_size();
            offset = height as f64 * 0.5;
        }

        // Move cursor to draw position and draw text.
        ctx.set_source_rgba(color.r, color.g, color.b, color.a);
        //ctx.move_to(pos.x + padding.left, pos.y + padding.top);
        // TODO: cleanup.
        ctx.move_to(pos.x, pos.y - offset);
        pangocairo::functions::show_layout(ctx, &self.layout);

        self.current_rect(pos, padding, vertical_center)
    }
}
